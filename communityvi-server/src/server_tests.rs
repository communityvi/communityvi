use crate::configuration::Configuration;
use crate::context::ApplicationContext;
use crate::message::client_request::{ChatRequest, RegisterRequest};
use crate::message::outgoing::broadcast_message::{
	BroadcastMessage, ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast, LeftReason,
};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::outgoing::success_message::SuccessMessage;
use crate::room::Room;
use crate::room::session_id::SessionId;
use crate::server::create_router;
use crate::utils::test_client::WebsocketTestClient;
use crate::utils::time_source::TimeSource;
use axum::http::StatusCode;
use axum::http::header::{CONNECTION, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE};
use js_int::uint;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::{WebSocketStream, tungstenite};

mod test_client;

use test_client::TestClient;

mod rest_api;

#[tokio::test]
async fn should_respond_to_websocket_messages() {
	let http_client = start_test_server().await;
	let mut websocket_client = websocket_test_client(&http_client).await;
	let session_id = register_client("Ferris", &mut websocket_client).await;
	assert_eq!(SessionId::from(0), session_id);
}

#[tokio::test]
async fn should_not_allow_invalid_messages_during_registration() {
	let http_client = start_test_server().await;
	let mut websocket_client = websocket_test_client(&http_client).await;
	let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8].into());
	websocket_client.send_raw(invalid_message).await;

	let response = websocket_client.receive_error_message(None).await;

	let expected_response = ErrorMessage::builder()
		.error(ErrorMessageType::InvalidFormat)
		.message(
			"Client request has incorrect message type. Message was: Binary(b\"\\x01\\x02\\x03\\x04\")".to_string(),
		)
		.build();
	assert_eq!(expected_response, response);
}

#[tokio::test]
async fn should_not_allow_invalid_messages_after_successful_registration() {
	let http_client = start_test_server().await;
	let (_session_id, mut websocket_client) = registered_websocket_test_client("Ferris", &http_client).await;
	let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8].into());
	websocket_client.send_raw(invalid_message).await;
	let response = websocket_client.receive_error_message(None).await;

	let expected_response = ErrorMessage::builder()
		.error(ErrorMessageType::InvalidFormat)
		.message(
			"Client request has incorrect message type. Message was: Binary(b\"\\x01\\x02\\x03\\x04\")".to_string(),
		)
		.build();
	assert_eq!(expected_response, response);
}

#[tokio::test]
async fn should_broadcast_messages() {
	let http_client = start_test_server().await;
	let message = r"Hello everyone \o/";
	let request = ChatRequest {
		message: message.to_string(),
	};
	let (alice_session_id, mut alice_test_client) = registered_websocket_test_client("Alice", &http_client).await;
	assert_eq!(SessionId::from(0), alice_session_id);
	let (bob_session_id, mut bob_test_client) = registered_websocket_test_client("Bob", &http_client).await;
	assert_eq!(SessionId::from(1), bob_session_id);

	let expected_bob_joined_broadcast = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
		id: bob_session_id,
		name: "Bob".to_string(),
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
}

#[tokio::test]
async fn should_broadcast_when_client_leaves_the_room() {
	let http_client = start_test_server().await;
	let (_alice_session_id, mut alice_client) = registered_websocket_test_client("Alice", &http_client).await;
	let (bob_session_id, bob_client) = registered_websocket_test_client("Bob", &http_client).await;

	let _bobs_join_message = alice_client.receive_broadcast_message().await;
	std::mem::drop(bob_client);

	let expected_leave_message = BroadcastMessage::ClientLeft(ClientLeftBroadcast {
		id: bob_session_id,
		name: "Bob".to_string(),
		reason: LeftReason::Closed,
	});
	let leave_message = alice_client.receive_broadcast_message().await;
	assert_eq!(expected_leave_message, leave_message);
}

#[tokio::test]
async fn test_server_should_upgrade_websocket_connection_and_ping_pong() {
	let http_client = start_test_server().await;
	let mut websocket_client = websocket_test_client(&http_client).await;
	websocket_client
		.send_raw(tungstenite::Message::Ping(Default::default()))
		.await;

	let pong = websocket_client.receive_raw().await;
	assert!(pong.is_pong());
}

#[tokio::test]
#[cfg(feature = "bundle-frontend")]
async fn test_server_should_serve_bundled_frontend() {
	use axum::http::StatusCode;

	let http_client = start_test_server().await;
	let response = http_client.get("/").send().await.expect("Request failed.");

	let status = response.status();
	let content = response.bytes().await.expect("Failed to collect bytes from response");

	assert_eq!(status, StatusCode::OK);
	assert!(content.starts_with(b"<!doctype html>"));
}

async fn registered_websocket_test_client(
	name: &'static str,
	http_client: &TestClient,
) -> (SessionId, WebsocketTestClient) {
	let mut websocket_client = websocket_test_client(http_client).await;
	let session_id = register_client(name, &mut websocket_client).await;
	(session_id, websocket_client)
}

async fn register_client(name: &str, test_client: &mut WebsocketTestClient) -> SessionId {
	let register_request = RegisterRequest { name: name.to_string() };

	let request_id = test_client.send_request(register_request).await;

	let response = test_client.receive_success_message(request_id).await;

	let SuccessMessage::Hello { id, .. } = response else {
		panic!("Expected Hello-Response, got '{response:?}'");
	};

	let joined_response = test_client.receive_broadcast_message().await;
	assert!(matches!(
		joined_response,
		BroadcastMessage::ClientJoined(ClientJoinedBroadcast { id: _, name: _ })
	));

	id
}

async fn websocket_test_client(http_client: &TestClient) -> WebsocketTestClient {
	let response = http_client
		.get("/ws")
		.header(CONNECTION, "upgrade")
		.header(UPGRADE, "websocket")
		.header(SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ==")
		.header(SEC_WEBSOCKET_VERSION, "13")
		.send()
		.await
		.expect("Websocket request failed.");
	assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

	let upgraded = response.upgrade().await.expect("Failed to upgrade client websocket.");
	WebSocketStream::from_raw_socket(upgraded, Role::Client, None)
		.await
		.into()
}

async fn start_test_server() -> TestClient {
	let configuration = Configuration {
		address: "127.0.0.1:8000".parse().unwrap(),
		log_filters: String::new(),
		room_size_limit: 10,
		heartbeat_interval: std::time::Duration::from_secs(2),
		missed_heartbeat_limit: 3,
	};
	let time_source = TimeSource::test();
	let application_context = ApplicationContext::new(configuration, time_source)
		.await
		.expect("Failed to create application context.");
	let test_room = application_context
		.repository
		.room()
		.create(
			application_context
				.database
				.connection()
				.await
				.expect("Database connection")
				.as_mut(),
			"test-room",
		)
		.await
		.expect("Could not create room");
	let room = Room::new(
		test_room.uuid,
		application_context.reference_timer.clone(),
		10,
		application_context.database.clone(),
		application_context.repository.clone(),
	);
	TestClient::new_with_host(create_router(application_context, room), "localhost")
		.await
		.expect("Failed to start test server")
}
