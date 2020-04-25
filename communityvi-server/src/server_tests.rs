use crate::configuration::Configuration;
use crate::message::{ClientRequest, OrderedMessage, ServerResponse};
use crate::room::client_id::ClientId;
use crate::room::Room;
use crate::server::{create_router, run_server};
use crate::utils::select_first_future::select_first_future;
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use gotham::plain::test::{TestConnect, TestServer};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use reqwest::StatusCode;
use std::convert::TryFrom;
use std::future::Future;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::runtime;
use tokio_tungstenite::tungstenite;
use url::Url;

const HOSTNAME_AND_PORT: &str = "localhost:8000";
lazy_static! {
	static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

type TestClient = gotham::test::TestClient<TestServer, TestConnect>;

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
	let url = Url::parse(&format!("ws://{}/ws", HOSTNAME_AND_PORT)).expect("Failed to build websocket URL");
	let (websocket_stream, _response) = tokio_tungstenite::connect_async(url)
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
		number: 0,
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
		number: _,
		message: ServerResponse::Hello { id },
	} = response
	{
		id
	} else {
		panic!("Expected Hello-Response, got '{:?}'", response);
	};

	let joined_response = response_stream.next().await.expect("Failed to get joined response.");
	assert!(matches!(joined_response, OrderedMessage {number: _, message: ServerResponse::Joined {id: _, name: _}}));

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
			tungstenite::Message::Text(r#"{"number":0,"type":"error","error":"invalid_format","message":"Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])"}"#.to_string());
		assert_eq!(expected_response, response);
	};
	test_future_with_running_server(future, false);
}

fn register_message_with_number(number: usize) -> String {
	format!(r#"{{"number":{},"type":"register","name":"Ferris"}}"#, number)
}

#[test]
fn should_not_allow_invalid_messages_after_successful_registration() {
	let future = async {
		let (mut sink, mut stream) = websocket_connection().await;
		let registration_message = tungstenite::Message::Text(register_message_with_number(0));
		sink.send(registration_message)
			.await
			.expect("Failed to send register message.");

		let hello_response = stream
			.next()
			.await
			.unwrap()
			.expect("Invalid websocket response received");
		let expected_hello_response = tungstenite::Message::Text(r#"{"number":0,"type":"hello","id":0}"#.to_string());
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
			tungstenite::Message::Text(r#"{"number":2,"type":"error","error":"invalid_format","message":"Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])"}"#.to_string());
		assert_eq!(expected_response, response);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_receive_pong_in_response_to_ping() {
	let future = async {
		let (client_id, mut sink, mut stream) = connect_and_register("Ferris".to_string()).await;
		assert_eq!(ClientId::from(0), client_id);
		let message = OrderedMessage {
			number: 42,
			message: ClientRequest::Ping,
		};
		sink.send(message).await.expect("Failed to sink message.");
		let message = stream.next().await.unwrap();
		assert_eq!(
			OrderedMessage {
				number: 2,
				message: ServerResponse::Pong,
			},
			message
		);
	};

	test_future_with_running_server(future, false);
}

#[test]
fn should_broadcast_messages() {
	let future = async move {
		let message = r#"Hello everyone \o/"#;
		let request = OrderedMessage {
			number: 1337,
			message: ClientRequest::Chat {
				message: message.to_string(),
			},
		};
		let (alice_client_id, mut alice_sink, mut alice_stream) = connect_and_register("Alice".to_string()).await;
		assert_eq!(ClientId::from(0), alice_client_id);
		let (bob_client_id, _bob_sink, mut bob_stream) = connect_and_register("Bob".to_string()).await;
		assert_eq!(ClientId::from(1), bob_client_id);

		let expected_bob_joined_response = OrderedMessage {
			number: 2,
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
				number: 3,
				message: expected_chat_response.clone()
			},
			alice_stream
				.next()
				.await
				.expect("Failed to receive response on client 1")
		);
		assert_eq!(
			OrderedMessage {
				number: 2,
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
			number: 3,
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
fn test_messages_should_have_sequence_numbers() {
	let future = async move {
		let first_request = OrderedMessage {
			number: 1,
			message: ClientRequest::Chat {
				message: "first".into(),
			},
		};
		let second_request = OrderedMessage {
			number: 2,
			message: ClientRequest::Chat {
				message: "second".into(),
			},
		};

		let (client_id, mut sink, mut stream) = connect_and_register("Charlie".to_string()).await;
		assert_eq!(ClientId::from(0), client_id);
		sink.send(first_request).await.expect("Failed to sink first message.");
		sink.send(second_request).await.expect("Failed to sink second message.");

		let first_response = stream.next().await.expect("Didn't receive first message");
		assert_eq!(
			OrderedMessage {
				number: 2,
				message: ServerResponse::Chat {
					sender_id: client_id,
					sender_name: "Charlie".to_string(),
					message: "first".into(),
				},
			},
			first_response
		);
		let second_response = stream.next().await.expect("Didn't receive second message");
		assert_eq!(
			OrderedMessage {
				number: 3,
				message: ServerResponse::Chat {
					sender_id: client_id,
					sender_name: "Charlie".to_string(),
					message: "second".into(),
				},
			},
			second_response
		);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn test_server_should_serve_reference_client_html_if_enabled() {
	let future = async {
		let url = Url::parse(&format!("http://{}/reference", HOSTNAME_AND_PORT)).unwrap();
		let response = reqwest::get(url).await.expect("Failed to request reference client.");
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
			"default-src 'none'; img-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'",
			content_security_policy
		);

		let response_text = response.text().await.expect("Incorrect response.");
		assert!(response_text.contains("html"));
	};
	test_future_with_running_server(future, true);
}

#[test]
fn test_server_should_serve_reference_client_javascript_if_enabled() {
	let test = |client: TestClient| {
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
	let future = async {
		let url = Url::parse(&format!("http://{}/reference", HOSTNAME_AND_PORT)).unwrap();
		let response = reqwest::get(url).await.expect("Failed to request reference client.");
		assert_eq!(StatusCode::NOT_FOUND, response.status());
	};
	test_future_with_running_server(future, false);
}

fn test_with_test_server(test: impl FnOnce(TestClient) -> (), enable_reference_client: bool) {
	let room = Room::new(10);
	let router = create_router(room, enable_reference_client);
	let server = gotham::test::TestServer::new(router).expect("Failed to build test server");
	let client = server.client();
	test(client);
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
