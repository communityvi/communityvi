use crate::configuration::Configuration;
use crate::message::{ClientRequest, OrderedMessage, ServerResponse};
use crate::room::client_id::ClientId;
use crate::room::Room;
use crate::server::{create_router, run_server};
use crate::utils::select_first_future::select_first_future;
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use gotham::hyper::http::header::{HeaderValue, SEC_WEBSOCKET_KEY, UPGRADE};
use gotham::hyper::http::StatusCode;
use gotham::hyper::upgrade::Upgraded;
use gotham::hyper::Body;
use gotham::plain::test::TestServer;
use gotham::test::Server;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::str::FromStr;
use tokio::runtime;
use tokio_tungstenite::{tungstenite, WebSocketStream};
use tungstenite::protocol::Role;

const HOSTNAME_AND_PORT: &str = "localhost:8000";
lazy_static! {
	static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

async fn typed_websocket_connection() -> (
	impl Sink<OrderedMessage<ClientRequest>, Error = ()>,
	impl Stream<Item = OrderedMessage<ServerResponse>>,
) {
	let (sink, stream) = websocket_connection().await;
	let stream = stream.map(|result| {
		let websocket_message = result.expect("Stream error.");
		let json = websocket_message.to_text().expect("No text message received.");
		OrderedMessage::<ServerResponse>::try_from(json).expect("Failed to parse JSON response")
	});
	let sink = sink.sink_map_err(|error| panic!("{}", error)).with(|message| {
		let websocket_message =
			tungstenite::Message::text(serde_json::to_string(&message).expect("Failed to convert message to JSON"));
		futures::future::ok(websocket_message)
	});
	(sink, stream)
}

async fn websocket_connection() -> (
	impl Sink<tungstenite::Message, Error = tungstenite::Error>,
	impl Stream<Item = Result<tungstenite::Message, tungstenite::Error>>,
) {
	let (websocket_stream, _response) = tokio_tungstenite::connect_async(format!("ws://{}/ws", HOSTNAME_AND_PORT))
		.await
		.map_err(|error| panic!("Websocket connection failed: {}", error))
		.unwrap();
	websocket_stream.split()
}

async fn register_client(
	name: String,
	request_sink: &mut (impl Sink<OrderedMessage<ClientRequest>, Error = ()> + Unpin),
	response_stream: &mut (impl Stream<Item = OrderedMessage<ServerResponse>> + Unpin),
) -> ClientId {
	let register_request = OrderedMessage {
		message: ClientRequest::Register { name: name.clone() },
	};

	request_sink
		.send(register_request)
		.await
		.expect("Failed to send register message.");

	let response = response_stream
		.next()
		.await
		.expect("Failed to get response to register request.");

	let id = if let OrderedMessage {
		message: ServerResponse::Hello { id, .. },
	} = response
	{
		id
	} else {
		panic!("Expected Hello-Response, got '{:?}'", response);
	};

	let joined_response = response_stream.next().await.expect("Failed to get joined response.");
	assert!(matches!(joined_response, OrderedMessage {message: ServerResponse::Joined {id: _, name: _}}));

	id
}

async fn connect_and_register(
	name: String,
) -> (
	ClientId,
	impl Sink<OrderedMessage<ClientRequest>, Error = ()>,
	impl Stream<Item = OrderedMessage<ServerResponse>>,
) {
	let (mut request_sink, mut response_stream) = typed_websocket_connection().await;
	let client_id = register_client(name, &mut request_sink, &mut response_stream).await;
	(client_id, request_sink, response_stream)
}

#[test]
fn should_respond_to_websocket_messages() {
	let future = async {
		let (mut sink, mut stream) = typed_websocket_connection().await;
		let client_id = register_client("Ferris".to_string(), &mut sink, &mut stream).await;
		assert_eq!(ClientId::from(0), client_id);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_not_allow_invalid_messages_during_registration() {
	let future = async {
		let (mut sink, mut stream) = websocket_connection().await;
		let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
		sink.send(invalid_message)
			.await
			.expect("Failed to send invalid message.");

		let response = stream
			.next()
			.await
			.unwrap()
			.expect("Invalid websocket response received");

		let expected_response =
			tungstenite::Message::Text(r#"{"type":"error","error":"invalid_format","message":"Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])"}"#.to_string());
		assert_eq!(expected_response, response);
	};
	test_future_with_running_server(future, false);
}

const REGISTER_MESSAGE: &str = r#"{"type":"register","name":"Ferris"}"#;

#[test]
fn should_not_allow_invalid_messages_after_successful_registration() {
	let future = async {
		let (mut sink, mut stream) = websocket_connection().await;
		let registration_message = tungstenite::Message::Text(REGISTER_MESSAGE.to_string());
		sink.send(registration_message)
			.await
			.expect("Failed to send register message.");

		let hello_response = stream
			.next()
			.await
			.unwrap()
			.expect("Invalid websocket response received");
		let expected_hello_response =
			tungstenite::Message::Text(r#"{"type":"hello","id":0,"current_medium":null}"#.to_string());
		assert_eq!(expected_hello_response, hello_response);

		let _ = stream.next().await.expect("Failed to receive joined response.");

		let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
		sink.send(invalid_message)
			.await
			.expect("Failed to send invalid message.");
		let response = stream
			.next()
			.await
			.unwrap()
			.expect("Invalid websocket response received");

		let expected_response =
			tungstenite::Message::Text(r#"{"type":"error","error":"invalid_format","message":"Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])"}"#.to_string());
		assert_eq!(expected_response, response);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_broadcast_messages() {
	let future = async move {
		let message = r#"Hello everyone \o/"#;
		let request = OrderedMessage {
			message: ClientRequest::Chat {
				message: message.to_string(),
			},
		};
		let (alice_client_id, mut alice_sink, mut alice_stream) = connect_and_register("Alice".to_string()).await;
		assert_eq!(ClientId::from(0), alice_client_id);
		let (bob_client_id, _bob_sink, mut bob_stream) = connect_and_register("Bob".to_string()).await;
		assert_eq!(ClientId::from(1), bob_client_id);

		let expected_bob_joined_response = OrderedMessage {
			message: ServerResponse::Joined {
				id: bob_client_id,
				name: "Bob".to_string(),
			},
		};
		let bob_joined_response = alice_stream.next().await.expect("Didn't get join message for Bob.");
		assert_eq!(expected_bob_joined_response, bob_joined_response);

		let expected_chat_response = ServerResponse::Chat {
			sender_id: alice_client_id,
			sender_name: "Alice".to_string(),
			message: message.to_string(),
		};

		alice_sink
			.send(request)
			.await
			.expect("Failed to sink broadcast message.");

		assert_eq!(
			OrderedMessage {
				message: expected_chat_response.clone()
			},
			alice_stream
				.next()
				.await
				.expect("Failed to receive response on client 1")
		);
		assert_eq!(
			OrderedMessage {
				message: expected_chat_response
			},
			bob_stream.next().await.expect("Failed to receive response on client 2")
		);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_broadcast_when_client_leaves_the_room() {
	let future = async {
		let (_alice_client_id, _alice_sink, mut alice_stream) = connect_and_register("Alice".to_string()).await;
		let (bob_client_id, bob_sink, bob_stream) = connect_and_register("Bob".to_string()).await;

		let _ = alice_stream.next().await; // skip join message for bob
		std::mem::drop(bob_sink);
		std::mem::drop(bob_stream);

		let expected_leave_message = OrderedMessage {
			message: ServerResponse::Left {
				id: bob_client_id,
				name: "Bob".to_string(),
			},
		};
		let leave_message = alice_stream.next().await.expect("Failed to get Leave message for bob");
		assert_eq!(expected_leave_message, leave_message);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn test_server_should_serve_reference_client_html_if_enabled() {
	let test = |server: &TestServer| {
		let client = server.client();
		let response = client
			.get("http://127.0.0.1:10000/reference")
			.perform()
			.expect("Failed to request reference_client.");
		assert_eq!(StatusCode::OK, response.status());
		let content_type = response
			.headers()
			.get("content-type")
			.expect("No content-type header.")
			.to_str()
			.expect("Content-Type header is no valid UTF-8");
		assert_eq!("text/html; charset=utf-8", content_type);

		let cache_control = response
			.headers()
			.get("cache-control")
			.expect("No cache-control header.")
			.to_str()
			.expect("Cache-Control header is no valid UTF-8");
		assert_eq!("no-cache", cache_control);

		let content_security_policy = response
			.headers()
			.get("content-security-policy")
			.expect("No cache-control header.")
			.to_str()
			.expect("Cache-Control header is no valid UTF-8");
		assert_eq!(
			"default-src 'none'; media-src 'self' blob:; img-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'",
			content_security_policy
		);

		let response_text = response.read_utf8_body().expect("Incorrect response.");
		assert!(response_text.contains("html"));
	};
	test_with_test_server(test, true);
}

#[test]
fn test_server_should_serve_reference_client_css_if_enabled() {
	let test = |server: &TestServer| {
		let client = server.client();
		let response = client
			.get("http://127.0.0.1:10000/reference/reference.css")
			.perform()
			.expect("Failed to request reference client css.");
		assert_eq!(StatusCode::OK, response.status());
		let content_type = response
			.headers()
			.get("content-type")
			.expect("No content-type header.")
			.to_str()
			.expect("Content-Type header is no valid UTF-8");
		assert_eq!("text/css; charset=utf-8", content_type);

		let cache_control = response
			.headers()
			.get("cache-control")
			.expect("No cache-control header.")
			.to_str()
			.expect("Cache-Control header is no valid UTF-8");
		assert_eq!("no-cache", cache_control);

		let response_text = response.read_utf8_body().expect("Incorrect response.");
		assert!(response_text.contains("width"));
	};
	test_with_test_server(test, true);
}

#[test]
fn test_server_should_serve_reference_client_javascript_if_enabled() {
	let test = |server: &TestServer| {
		let client = server.client();
		let response = client
			.get("http://127.0.0.1:10000/reference/reference.js")
			.perform()
			.expect("Failed to request reference client javascript.");
		assert_eq!(StatusCode::OK, response.status());
		let content_type = response
			.headers()
			.get("content-type")
			.expect("No content-type header.")
			.to_str()
			.expect("Content-Type header is no valid UTF-8");
		assert_eq!("application/javascript; charset=utf-8", content_type);

		let cache_control = response
			.headers()
			.get("cache-control")
			.expect("No cache-control header.")
			.to_str()
			.expect("Cache-Control header is no valid UTF-8");
		assert_eq!("no-cache", cache_control);

		let response_text = response.read_utf8_body().expect("Incorrect response.");
		assert!(response_text.contains("use strict"));
	};
	test_with_test_server(test, true);
}

#[test]
fn server_should_respond_to_websocket_pings() {
	let future = async {
		let (mut sink, mut stream) = websocket_connection().await;
		let ping_content = vec![1u8, 9, 8, 0];
		sink.send(tungstenite::Message::Ping(ping_content.clone()))
			.await
			.expect("Failed to send ping.");
		let received_pong = stream.next().await.unwrap().expect("Failed to receive pong.");
		let expected_pong = tungstenite::Message::Pong(ping_content);
		assert_eq!(expected_pong, received_pong);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn test_server_should_not_serve_reference_client_if_disabled() {
	let test = |server: &TestServer| {
		let client = server.client();
		let response = client
			.get("http://127.0.0.1:10000/reference")
			.perform()
			.expect("Failed to request reference client.");
		assert_eq!(StatusCode::NOT_FOUND, response.status());
	};
	test_with_test_server(test, false);
}

#[derive(Debug)]
enum ImpossibleError {}
impl Display for ImpossibleError {
	fn fmt(&self, _formatter: &mut Formatter) -> std::fmt::Result {
		Ok(())
	}
}
impl Error for ImpossibleError {}

#[test]
fn test_server_should_upgrade_websocket_connection_and_ping_pong() {
	let test = |server: &TestServer| {
		let mut websocket = connect_to_test_server_websocket(server);
		let future = async move {
			websocket
				.send(tungstenite::Message::Ping(vec![]))
				.await
				.expect("Failed to send ping.");

			let pong = websocket
				.next()
				.await
				.expect("Websocket ended prematurely")
				.expect("Didn't receive Pong.");
			assert!(pong.is_pong());
		};
		run_future_on_test_server(future, server)
	};
	test_with_test_server(test, false);
}

fn connect_to_test_server_websocket(server: &TestServer) -> WebSocketStream<Upgraded> {
	let client = server.client();

	let mut request = client.get("ws://127.0.0.1:10000/ws");
	let headers = request.headers_mut();
	headers.insert(UPGRADE, HeaderValue::from_static("websocket"));
	headers.insert(SEC_WEBSOCKET_KEY, HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="));

	let mut response = client
		.perform(request)
		.expect("Failed to initiate websocket connection.");
	// We don't own the `TestRespons`'s `Body`, so we need to swap it out for an empty one ...
	let mut body = Body::empty();
	std::mem::swap(&mut body, response.deref_mut().body_mut());

	server
		.run_future(async move {
			let upgraded = body.on_upgrade().await.expect("Failed to upgrade connection");
			let websocket_stream = WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await;
			Ok::<_, ImpossibleError>(websocket_stream) // `test::Server::run_future` requires `Result` with `Error` for no apparent reason
		})
		.unwrap() // wrap the `ImpossibleError` away, whoooosh
}

fn run_future_on_test_server<FutureType, Output>(future: FutureType, server: &TestServer) -> Output
where
	Output: Send + 'static,
	FutureType: Future<Output = Output> + Send + 'static,
{
	server.run_future(future.map(Ok::<_, ImpossibleError>)).unwrap()
}

fn test_with_test_server(test: impl FnOnce(&TestServer) -> (), enable_reference_client: bool) {
	let room = Room::new(10);
	let router = create_router(room, enable_reference_client);
	let server = gotham::test::TestServer::new(router).expect("Failed to build test server");
	test(&server);
}

fn test_future_with_running_server<OutputType, FutureType>(
	future_to_test: FutureType,
	enable_reference_client: bool,
) -> OutputType
where
	OutputType: Send + 'static,
	FutureType: Future<Output = OutputType> + Send + 'static,
{
	let _guard = TEST_MUTEX.lock();
	let mut runtime = runtime::Builder::new()
		.threaded_scheduler()
		.enable_all()
		.build()
		.expect("Failed to create runtime");
	let (sender, receiver) = futures::channel::oneshot::channel();
	let receiver = receiver.then(|_| futures::future::ready(()));
	let configuration = Configuration {
		address: SocketAddr::from_str("127.0.0.1:8000").unwrap(),
		log_filters: "debug".to_string(),
		room_size_limit: 10,
	};
	let server = async move {
		select_first_future(receiver, run_server(&configuration, enable_reference_client)).await;
	};
	let server_handle = runtime.spawn(server);

	let output = runtime.block_on(future_to_test);
	sender.send(()).expect("Failed to send shutdown.");
	runtime.block_on(server_handle).expect("Failed to join server.");
	output
}
