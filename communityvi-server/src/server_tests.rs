use crate::configuration::Configuration;
use crate::context::ApplicationContext;
use crate::message::client_request::{ChatRequest, RegisterRequest};
use crate::message::outgoing::broadcast_message::{
	BroadcastMessage, ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast, LeftReason,
};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::outgoing::success_message::SuccessMessage;
use crate::reference_time::ReferenceTimer;
use crate::room::client_id::ClientId;
use crate::room::Room;
use crate::server::create_filter;
use crate::utils::test_client::WebsocketTestClient;
use crate::utils::time_source::TimeSource;
use js_int::uint;
use rweb::http::header::{CONNECTION, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE};
use rweb::http::StatusCode;
use rweb::hyper::service::make_service_fn;
use rweb::hyper::upgrade;
use rweb::service;
use std::convert::Infallible;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::{tungstenite, WebSocketStream};

type TestClient = hyper_test::Client;

mod rest_api;

#[tokio::test]
async fn should_respond_to_websocket_messages() {
	let http_client = start_test_server();
	let mut test_client = websocket_test_client(&http_client).await;
	let client_id = register_client("Ferris", &mut test_client).await;
	assert_eq!(ClientId::from(0), client_id);
}

#[tokio::test]
async fn should_not_allow_invalid_messages_during_registration() {
	let http_client = start_test_server();
	let mut test_client = websocket_test_client(&http_client).await;
	let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
	test_client.send_raw(invalid_message).await;

	let response = test_client.receive_error_message(None).await;

	let expected_response = ErrorMessage::builder()
		.error(ErrorMessageType::InvalidFormat)
		.message("Client request has incorrect message type. Message was: Binary([1, 2, 3, 4])".to_string())
		.build();
	assert_eq!(expected_response, response);
}

#[tokio::test]
async fn should_not_allow_invalid_messages_after_successful_registration() {
	let http_client = start_test_server();
	let (_client_id, mut test_client) = registered_websocket_test_client("Ferris", &http_client).await;
	let invalid_message = tungstenite::Message::Binary(vec![1u8, 2u8, 3u8, 4u8]);
	test_client.send_raw(invalid_message).await;
	let response = test_client.receive_error_message(None).await;

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
	let (alice_client_id, mut alice_test_client) = registered_websocket_test_client("Alice", &http_client).await;
	assert_eq!(ClientId::from(0), alice_client_id);
	let (bob_client_id, mut bob_test_client) = registered_websocket_test_client("Bob", &http_client).await;
	assert_eq!(ClientId::from(1), bob_client_id);

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
	let http_client = start_test_server();
	let (_alice_client_id, mut alice_test_client) = registered_websocket_test_client("Alice", &http_client).await;
	let (bob_client_id, bob_test_client) = registered_websocket_test_client("Bob", &http_client).await;

	let _bobs_join_message = alice_test_client.receive_broadcast_message().await;
	std::mem::drop(bob_test_client);

	let expected_leave_message = BroadcastMessage::ClientLeft(ClientLeftBroadcast {
		id: bob_client_id,
		name: "Bob".to_string(),
		reason: LeftReason::Closed,
	});
	let leave_message = alice_test_client.receive_broadcast_message().await;
	assert_eq!(expected_leave_message, leave_message);
}

#[tokio::test]
async fn test_server_should_upgrade_websocket_connection_and_ping_pong() {
	let http_client = start_test_server();
	let mut test_client = websocket_test_client(&http_client).await;
	test_client.send_raw(tungstenite::Message::Ping(vec![])).await;

	let pong = test_client.receive_raw().await;
	assert!(pong.is_pong());
}

#[tokio::test]
#[cfg(feature = "bundle-frontend")]
async fn test_server_should_serve_bundled_frontend() {
	use rweb::hyper::StatusCode;

	let http_client = start_test_server();
	let mut response = http_client.get("/").send().await.expect("Request failed.");

	let content = response.content().await.expect("Failed to get responsed bytes");
	assert_eq!(response.status(), StatusCode::OK);
	assert!(content.starts_with(b"<!DOCTYPE html>"));
}

async fn registered_websocket_test_client(
	name: &'static str,
	http_client: &TestClient,
) -> (ClientId, WebsocketTestClient) {
	let mut test_client = websocket_test_client(http_client).await;
	let client_id = register_client(name, &mut test_client).await;
	(client_id, test_client)
}

async fn register_client(name: &str, test_client: &mut WebsocketTestClient) -> ClientId {
	let register_request = RegisterRequest { name: name.to_string() };

	let request_id = test_client.send_request(register_request).await;

	let response = test_client.receive_success_message(request_id).await;

	let id = if let SuccessMessage::Hello { id, .. } = response {
		id
	} else {
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
		.get("ws://localhost/ws")
		.header(CONNECTION, "upgrade")
		.header(UPGRADE, "websocket")
		.header(SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ==")
		.header(SEC_WEBSOCKET_VERSION, "13")
		.send()
		.await
		.expect("Websocket request failed.");
	assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

	let upgraded = upgrade::on(response.into_response())
		.await
		.expect("Failed to upgrade client websocket.");
	WebSocketStream::from_raw_socket(upgraded, Role::Client, None)
		.await
		.into()
}

pub(self) fn start_test_server() -> TestClient {
	let room = Room::new(ReferenceTimer::default(), 10);
	let configuration = Configuration {
		address: "127.0.0.1:8000".parse().unwrap(),
		log_filters: String::new(),
		room_size_limit: 10,
		heartbeat_interval: std::time::Duration::from_secs(2),
		missed_heartbeat_limit: 3,
	};
	let time_source = TimeSource::test();
	let application_context = ApplicationContext::new(configuration, time_source);

	let service = service(create_filter(application_context, room));
	let make_service = make_service_fn(move |_| {
		let service = service.clone();
		async move { Ok::<_, Infallible>(service) }
	});
	hyper_test::Client::new_with_host(make_service, "localhost").expect("Failed to start test server")
}
