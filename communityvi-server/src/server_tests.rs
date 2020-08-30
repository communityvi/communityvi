use crate::configuration::Configuration;
use crate::context::ApplicationContext;
use crate::message::client_request::{ChatRequest, RegisterRequest};
use crate::message::outgoing::broadcast_message::{
	BroadcastMessage, ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast, LeftReason,
};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::outgoing::success_message::SuccessMessage;
use crate::room::client_id::ClientId;
use crate::room::Room;
use crate::server::create_router;
use crate::utils::test_client::WebsocketTestClient;
use crate::utils::time_source::TimeSource;
use gotham::hyper::http::header::{HeaderValue, SEC_WEBSOCKET_KEY, UPGRADE};
use gotham::hyper::http::StatusCode;
use gotham::hyper::Body;
use gotham::plain::test::TestServer;
use gotham::test::Server;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::DerefMut;
use tokio_tungstenite::{tungstenite, WebSocketStream};
use tungstenite::protocol::Role;

const TEST_SERVER_URL: &str = "127.0.0.1:10000";

#[test]
fn should_respond_to_websocket_messages() {
	let test = |server: &TestServer| {
		let mut test_client = websocket_test_client(server);
		let future = async move {
			let client_id = register_client("Ferris", &mut test_client).await;
			assert_eq!(ClientId::from(0), client_id);
		};
		server.run_future(future);
	};
	test_with_test_server(test, false);
}

#[test]
fn should_not_allow_invalid_messages_during_registration() {
	let test = |server: &TestServer| {
		let mut test_client = websocket_test_client(server);
		let future = async move {
			let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
			test_client.send_raw(invalid_message).await;

			let response = test_client.receive_error_message(None).await;

			let expected_response = ErrorMessage::builder()
				.error(ErrorMessageType::InvalidFormat)
				.message("Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])".to_string())
				.build();
			assert_eq!(expected_response, response);
		};
		server.run_future(future);
	};
	test_with_test_server(test, false);
}

#[test]
fn should_not_allow_invalid_messages_after_successful_registration() {
	let test = |server: &TestServer| {
		let (_client_id, mut test_client) = registered_websocket_test_client("Ferris", server);

		let future = async move {
			let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
			test_client.send_raw(invalid_message).await;
			let response = test_client.receive_error_message(None).await;

			let expected_response = ErrorMessage::builder()
				.error(ErrorMessageType::InvalidFormat)
				.message("Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])".to_string())
				.build();
			assert_eq!(expected_response, response);
		};
		server.run_future(future);
	};
	test_with_test_server(test, false);
}

#[test]
fn should_broadcast_messages() {
	let test = |server: &TestServer| {
		let message = r#"Hello everyone \o/"#;
		let request = ChatRequest {
			message: message.to_string(),
		};
		let (alice_client_id, mut alice_test_client) = registered_websocket_test_client("Alice", server);
		assert_eq!(ClientId::from(0), alice_client_id);
		let (bob_client_id, mut bob_test_client) = registered_websocket_test_client("Bob", server);
		assert_eq!(ClientId::from(1), bob_client_id);

		let future = async move {
			let expected_bob_joined_broadcast = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
				id: bob_client_id,
				name: "Bob".to_string(),
			});
			let bob_joined_broadcast = alice_test_client.receive_broadcast_message().await;
			assert_eq!(expected_bob_joined_broadcast, bob_joined_broadcast);

			let expected_chat_broadcast = BroadcastMessage::Chat(ChatBroadcast {
				sender_id: alice_client_id,
				sender_name: "Alice".to_string(),
				message: message.to_string(),
				counter: 0,
			});

			let request_id = alice_test_client.send_request(request).await;
			assert_eq!(
				SuccessMessage::Success,
				alice_test_client.receive_success_message(request_id).await
			);

			assert_eq!(
				expected_chat_broadcast,
				alice_test_client.receive_broadcast_message().await
			);
			assert_eq!(
				expected_chat_broadcast,
				bob_test_client.receive_broadcast_message().await
			);
		};
		server.run_future(future);
	};
	test_with_test_server(test, false);
}

#[test]
fn should_broadcast_when_client_leaves_the_room() {
	let test = |server: &TestServer| {
		let (_alice_client_id, mut alice_test_client) = registered_websocket_test_client("Alice", server);
		let (bob_client_id, bob_test_client) = registered_websocket_test_client("Bob", server);

		let future = async move {
			let _ = alice_test_client.receive_broadcast_message().await; // skip join message for bob
			std::mem::drop(bob_test_client);

			let expected_leave_message = BroadcastMessage::ClientLeft(ClientLeftBroadcast {
				id: bob_client_id,
				name: "Bob".to_string(),
				reason: LeftReason::Closed,
			});
			let leave_message = alice_test_client.receive_broadcast_message().await;
			assert_eq!(expected_leave_message, leave_message);
		};
		server.run_future(future);
	};
	test_with_test_server(test, false);
}

#[test]
fn test_server_should_serve_reference_client_html_if_enabled() {
	let test = |server: &TestServer| {
		let client = server.client();
		let response = client
			.get(format!("http://{}/reference", TEST_SERVER_URL))
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
			.get(format!("http://{}/reference/reference.css", TEST_SERVER_URL))
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
			.get(format!("http://{}/reference/reference.js", TEST_SERVER_URL))
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
fn test_server_should_not_serve_reference_client_if_disabled() {
	let test = |server: &TestServer| {
		let client = server.client();
		let response = client
			.get(format!("http://{}/reference", TEST_SERVER_URL))
			.perform()
			.expect("Failed to request reference client.");
		assert_eq!(StatusCode::NOT_FOUND, response.status());
	};
	test_with_test_server(test, false);
}

#[derive(Debug)]
#[allow(clippy::empty_enum)]
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
		let mut test_client = websocket_test_client(server);
		let future = async move {
			test_client.send_raw(tungstenite::Message::Ping(vec![])).await;

			let pong = test_client.receive_raw().await;
			assert!(pong.is_pong());
		};
		server.run_future(future);
	};
	test_with_test_server(test, false);
}

fn registered_websocket_test_client(name: &'static str, server: &TestServer) -> (ClientId, WebsocketTestClient) {
	let mut test_client = websocket_test_client(server);
	let register_future = async move {
		let client_id = { register_client(name, &mut test_client).await };
		(client_id, test_client)
	};
	server.run_future(register_future)
}

async fn register_client(name: &str, test_client: &mut WebsocketTestClient) -> ClientId {
	let register_request = RegisterRequest { name: name.to_string() };

	let request_id = test_client.send_request(register_request).await;

	let response = test_client.receive_success_message(request_id).await;

	let id = if let SuccessMessage::Hello { id, .. } = response {
		id
	} else {
		panic!("Expected Hello-Response, got '{:?}'", response);
	};

	let joined_response = test_client.receive_broadcast_message().await;
	assert!(matches!(
		joined_response,
		BroadcastMessage::ClientJoined(ClientJoinedBroadcast { id: _, name: _ })
	));

	id
}

fn websocket_test_client(server: &TestServer) -> WebsocketTestClient {
	let client = server.client();

	let mut request = client.get(format!("ws://{}/ws", TEST_SERVER_URL));
	let headers = request.headers_mut();
	headers.insert(UPGRADE, HeaderValue::from_static("websocket"));
	headers.insert(SEC_WEBSOCKET_KEY, HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="));

	let mut response = client
		.perform(request)
		.expect("Failed to initiate websocket connection.");
	// We don't own the `TestResponse`'s `Body`, so we need to swap it out for an empty one ...
	let mut body = Body::empty();
	std::mem::swap(&mut body, response.deref_mut().body_mut());

	let websocket = server.run_future(async move {
		let upgraded = body.on_upgrade().await.expect("Failed to upgrade connection");
		WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await
	});

	websocket.into()
}

fn test_with_test_server(test: impl FnOnce(&TestServer), enable_reference_client: bool) {
	let room = Room::new(10);
	let configuration = Configuration {
		address: "127.0.0.1:8000".parse().unwrap(),
		log_filters: "".to_string(),
		room_size_limit: 10,
		heartbeat_interval: std::time::Duration::from_secs(2),
		missed_heartbeat_limit: 3,
	};
	let time_source = TimeSource::test();
	let application_context = ApplicationContext::new(configuration, time_source);
	let router = create_router(application_context, room, enable_reference_client);
	let server = gotham::test::TestServer::new(router).expect("Failed to build test server");
	test(&server);
}
