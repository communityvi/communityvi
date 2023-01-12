use crate::configuration::Configuration;
use crate::context::ApplicationContext;
use crate::message::client_request::ChatRequest;
use crate::message::outgoing::broadcast_message::{
	BroadcastMessage, ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast, LeftReason, Participant,
};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::room::session_id::SessionId;
use crate::room::Room;
use crate::server::create_router;
use crate::utils::test_client::WebsocketTestClient;
use crate::utils::time_source::TimeSource;
use axum::http::header::{CONNECTION, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE};
use axum::http::StatusCode;
use hyper_test::hyper;
use hyper_test::hyper::upgrade;
use js_int::uint;
use serde_json::json;
use std::collections::BTreeSet;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::{tungstenite, WebSocketStream};

type TestClient = hyper_test::Client;

mod rest_api;

#[tokio::test]
async fn should_not_allow_invalid_messages_after_successful_registration() {
	let http_client = start_test_server();
	let (_session_id, mut websocket_client) = registered_websocket_test_client("Ferris", &http_client).await;
	let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
	websocket_client.send_raw(invalid_message).await;
	let response = websocket_client.receive_error_message(None).await;

	let expected_response = ErrorMessage::builder()
		.error(ErrorMessageType::InvalidFormat)
		.message("Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])".to_string())
		.build();
	assert_eq!(expected_response, response);
}

#[tokio::test]
async fn should_broadcast_messages() {
	let http_client = start_test_server();
	let message = r#"Hello everyone \o/"#;
	let request = ChatRequest {
		message: message.to_string(),
	};
	let (alice_session_id, mut alice_test_client) = registered_websocket_test_client("Alice", &http_client).await;
	assert_eq!(SessionId::from(0), alice_session_id);
	let (bob_session_id, mut bob_test_client) = registered_websocket_test_client("Bob", &http_client).await;
	assert_eq!(SessionId::from(1), bob_session_id);

	let bob = Participant::new(bob_session_id, "Bob".to_string());
	let alice = Participant::new(alice_session_id, "Alice".to_string());

	let expected_bob_joined_broadcast = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
		id: bob_session_id,
		name: "Bob".to_string(),
		participants: BTreeSet::from_iter([alice, bob]),
	});
	let bob_joined_broadcast = alice_test_client.receive_broadcast_message().await;
	assert_eq!(expected_bob_joined_broadcast, bob_joined_broadcast);

	let expected_chat_broadcast = BroadcastMessage::Chat(ChatBroadcast {
		sender_id: alice_session_id,
		sender_name: "Alice".to_string(),
		message: message.to_string(),
		counter: uint!(0),
	});

	let request_id = alice_test_client.send_request(request).await;
	alice_test_client.receive_success_message(request_id).await;

	assert_eq!(
		expected_chat_broadcast,
		alice_test_client.receive_broadcast_message().await
	);
	assert_eq!(
		expected_chat_broadcast,
		bob_test_client.receive_broadcast_message().await
	);
}

#[tokio::test]
async fn should_broadcast_when_client_leaves_the_room() {
	let http_client = start_test_server();
	let (alice_session_id, mut alice_client) = registered_websocket_test_client("Alice", &http_client).await;
	let alice = Participant::new(alice_session_id, "Alice".to_string());
	let (bob_session_id, bob_client) = registered_websocket_test_client("Bob", &http_client).await;

	let _bobs_join_message = alice_client.receive_broadcast_message().await;
	std::mem::drop(bob_client);

	let expected_leave_message = BroadcastMessage::ClientLeft(ClientLeftBroadcast {
		id: bob_session_id,
		name: "Bob".to_string(),
		reason: LeftReason::Closed,
		participants: BTreeSet::from_iter([alice]),
	});
	let leave_message = alice_client.receive_broadcast_message().await;
	assert_eq!(expected_leave_message, leave_message);
}

#[tokio::test]
async fn test_server_should_upgrade_websocket_connection_and_ping_pong() {
	let http_client = start_test_server();
	let (_, mut websocket_client) = registered_websocket_test_client("test", &http_client).await;
	websocket_client.send_raw(tungstenite::Message::Ping(vec![])).await;

	let pong = websocket_client.receive_raw().await;
	assert!(pong.is_pong());
}

#[tokio::test]
#[cfg(feature = "bundle-frontend")]
async fn test_server_should_serve_bundled_frontend() {
	use axum::http::StatusCode;

	let http_client = start_test_server();
	let mut response = http_client.get("/").send().await.expect("Request failed.");

	let content = response.content().await.expect("Failed to collect bytes from response");

	assert_eq!(response.status(), StatusCode::OK);
	assert!(content.starts_with(b"<!DOCTYPE html>"));
}

async fn registered_websocket_test_client(
	name: &'static str,
	http_client: &TestClient,
) -> (SessionId, WebsocketTestClient) {
	// Register
	let register_response = http_client
		.post("http://localhost/api/user")
		.json(&json!({ "name": name }))
		.send()
		.await
		.expect("Registration request failed.");
	assert!(register_response.status().is_success(), "Registration failed");

	// Login
	let login_body = http_client
		.post("http://localhost/api/login")
		.json(&json!({ "username": name }))
		.send()
		.await
		.expect("Login request failed.")
		.into_body();
	let body_bytes = hyper::body::to_bytes(login_body)
		.await
		.expect("Could not extract body from login request.");
	let token = serde_json::from_slice::<String>(&body_bytes).expect("Could not deserialize token.");

	// Start session
	let response = http_client
		.get("ws://localhost/api/room/default/join")
		.bearer_auth(token)
		.header(CONNECTION, "upgrade")
		.header(UPGRADE, "websocket")
		.header(SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ==")
		.header(SEC_WEBSOCKET_VERSION, "13")
		.send()
		.await
		.expect("Session start request failed.");
	assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
	let upgraded = upgrade::on(response.into_response())
		.await
		.expect("Failed to upgrade client websocket.");
	let mut websocket_client: WebsocketTestClient = WebSocketStream::from_raw_socket(upgraded, Role::Client, None)
		.await
		.into();

	let joined_response = websocket_client.receive_broadcast_message().await;
	let BroadcastMessage::ClientJoined(ClientJoinedBroadcast { id: session_id, name: _, participants: _ }) = joined_response else {
		panic!("Received message was not ClientJoined broadcast.");
	};

	(session_id, websocket_client)
}

pub(self) fn start_test_server() -> TestClient {
	let configuration = Configuration::test();
	let time_source = TimeSource::test();
	let application_context =
		ApplicationContext::new(configuration, time_source).expect("ApplicationContext failed to initialize.");
	let room = Room::new(application_context.reference_timer.clone(), 10);
	hyper_test::Client::new_with_host(
		create_router(application_context, room).into_make_service(),
		"localhost",
	)
	.expect("Failed to start test server")
}
