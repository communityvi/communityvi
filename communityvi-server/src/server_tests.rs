use crate::configuration::Configuration;
use crate::message::broadcast::Broadcast;
use crate::message::broadcast::{ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast};
use crate::message::client_request::{ChatRequest, RegisterRequest};
use crate::message::server_response::{ErrorResponse, ErrorResponseType, HelloResponse, ServerResponse};
use crate::room::client_id::ClientId;
use crate::room::Room;
use crate::server::{create_router, run_server};
use crate::utils::select_first_future::select_first_future;
use crate::utils::test_client::WebsocketTestClient;
use futures::FutureExt;
use gotham::hyper::http::header::{HeaderValue, SEC_WEBSOCKET_KEY, UPGRADE};
use gotham::hyper::http::StatusCode;
use gotham::hyper::Body;
use gotham::plain::test::TestServer;
use gotham::test::Server;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime;
use tokio_tungstenite::{tungstenite, WebSocketStream};
use tungstenite::protocol::Role;

const HOSTNAME_AND_PORT: &str = "localhost:8000";
lazy_static! {
	static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

async fn websocket_connection() -> WebsocketTestClient {
	let (websocket_stream, _response) = tokio_tungstenite::connect_async(format!("ws://{}/ws", HOSTNAME_AND_PORT))
		.await
		.map_err(|error| panic!("Websocket connection failed: {}", error))
		.unwrap();
	websocket_stream.into()
}

async fn register_client(name: &str, test_client: &mut WebsocketTestClient) -> ClientId {
	let register_request = RegisterRequest { name: name.to_string() };

	test_client.send_request(register_request).await;

	let response = test_client.receive_response().await;

	let id = if let ServerResponse::Hello(HelloResponse { id, .. }) = response {
		id
	} else {
		panic!("Expected Hello-Response, got '{:?}'", response);
	};

	let joined_response = test_client.receive_broadcast().await;
	assert!(matches!(
		joined_response,
		Broadcast::ClientJoined(ClientJoinedBroadcast { id: _, name: _ })
	));

	id
}

async fn connect_and_register(name: &str) -> (ClientId, WebsocketTestClient) {
	let mut test_client = websocket_connection().await;
	let client_id = register_client(name, &mut test_client).await;
	(client_id, test_client)
}

#[test]
fn should_respond_to_websocket_messages() {
	let future = async {
		let mut test_client = websocket_connection().await;
		let client_id = register_client("Ferris", &mut test_client).await;
		assert_eq!(ClientId::from(0), client_id);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_not_allow_invalid_messages_during_registration() {
	let future = async {
		let mut test_client = websocket_connection().await;
		let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
		test_client.send_raw(invalid_message).await;

		let response = test_client.receive_response().await;

		let expected_response = ServerResponse::Error(ErrorResponse {
			error: ErrorResponseType::InvalidFormat,
			message: "Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])".to_string(),
		});
		assert_eq!(expected_response, response);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_not_allow_invalid_messages_after_successful_registration() {
	let future = async {
		let (_client_id, mut test_client) = connect_and_register("Ferris").await;

		let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
		test_client.send_raw(invalid_message).await;
		let response = test_client.receive_response().await;

		let expected_response = ServerResponse::Error(ErrorResponse {
			error: ErrorResponseType::InvalidFormat,
			message: "Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])".to_string(),
		});
		assert_eq!(expected_response, response);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_broadcast_messages() {
	let future = async move {
		let message = r#"Hello everyone \o/"#;
		let request = ChatRequest {
			message: message.to_string(),
		};
		let (alice_client_id, mut alice_test_client) = connect_and_register("Alice").await;
		assert_eq!(ClientId::from(0), alice_client_id);
		let (bob_client_id, mut bob_test_client) = connect_and_register("Bob").await;
		assert_eq!(ClientId::from(1), bob_client_id);

		let expected_bob_joined_broadcast = Broadcast::ClientJoined(ClientJoinedBroadcast {
			id: bob_client_id,
			name: "Bob".to_string(),
		});
		let bob_joined_broadcast = alice_test_client.receive_broadcast().await;
		assert_eq!(expected_bob_joined_broadcast, bob_joined_broadcast);

		let expected_chat_broadcast = Broadcast::Chat(ChatBroadcast {
			sender_id: alice_client_id,
			sender_name: "Alice".to_string(),
			message: message.to_string(),
		});

		alice_test_client.send_request(request).await;

		assert_eq!(expected_chat_broadcast, alice_test_client.receive_broadcast().await);
		assert_eq!(expected_chat_broadcast, bob_test_client.receive_broadcast().await);
	};
	test_future_with_running_server(future, false);
}

#[test]
fn should_broadcast_when_client_leaves_the_room() {
	let future = async {
		let (_alice_client_id, mut alice_test_client) = connect_and_register("Alice").await;
		let (bob_client_id, bob_test_client) = connect_and_register("Bob").await;

		let _ = alice_test_client.receive_broadcast().await; // skip join message for bob
		std::mem::drop(bob_test_client);

		let expected_leave_message = Broadcast::ClientLeft(ClientLeftBroadcast {
			id: bob_client_id,
			name: "Bob".to_string(),
		});
		let leave_message = alice_test_client.receive_broadcast().await;
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
		let mut test_client = websocket_connection().await;
		let ping_content = vec![1u8, 9, 8, 0];
		test_client
			.send_raw(tungstenite::Message::Ping(ping_content.clone()))
			.await;
		let received_pong = test_client.receive_raw().await;
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
		let test_client = websocket_test_client(server);
		let future = async move {
			test_client
				.lock()
				.await
				.send_raw(tungstenite::Message::Ping(vec![]))
				.await;

			let pong = test_client.lock().await.receive_raw().await;
			assert!(pong.is_pong());
		};
		run_future_on_test_server(future, server)
	};
	test_with_test_server(test, false);
}

fn websocket_test_client(server: &TestServer) -> Arc<tokio::sync::Mutex<WebsocketTestClient>> {
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

	let websocket = server
		.run_future(async move {
			let upgraded = body.on_upgrade().await.expect("Failed to upgrade connection");
			let websocket_stream = WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await;
			Ok::<_, ImpossibleError>(websocket_stream) // `test::Server::run_future` requires `Result` with `Error` for no apparent reason
		})
		.unwrap(); // wrap the `ImpossibleError` away, whoooosh
	Arc::new(tokio::sync::Mutex::new(websocket.into()))
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
