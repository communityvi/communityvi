use crate::connection::receiver::{MessageReceiver, ReceivedMessage};
use crate::connection::sender::MessageSender;
use crate::context::ApplicationContext;
use crate::message::client_request::{
	ChatRequest, ClientRequest, InsertMediumRequest, PauseRequest, PlayRequest, RegisterRequest,
};
use crate::message::outgoing::broadcast_message::{
	ClientJoinedBroadcast, ClientLeftBroadcast, LeftReason, MediumStateChangedBroadcast, VersionedMediumBroadcast,
};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::outgoing::success_message::{ClientResponse, SuccessMessage};
use crate::room::client::Client;
use crate::room::error::RoomError;
use crate::room::medium::Medium;
use crate::room::Room;
use crate::utils::portable_unsigned_integer::PortableUnsignedInteger;
use crate::utils::time_source::TimeSource;
use chrono::Duration;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use governor::{Quota, RateLimiter};
use log::{debug, error, info};
use nonzero_ext::nonzero;

/// Once this count of heartbeats are missed, the client is kicked.
const MISSED_HEARTBEAT_LIMIT: u32 = 3;

pub async fn run_client(
	application_context: ApplicationContext,
	room: Room,
	message_sender: MessageSender,
	message_receiver: MessageReceiver,
) {
	if let Some((client, message_receiver)) = register_client(room.clone(), message_sender, message_receiver).await {
		let client_id = client.id();
		let client_name = client.name().to_string();
		let (pong_sender, pong_receiver) = mpsc::channel(MISSED_HEARTBEAT_LIMIT as usize);

		let left_reason = tokio::select! {
			_ = handle_messages(&room, client.clone(), message_receiver, pong_sender) => LeftReason::Closed,
			_ = send_broadcasts(client.clone()) => LeftReason::Closed,
			left_reason = heartbeat(
				client,
				&application_context.time_source,
				pong_receiver,
				application_context.configuration.heartbeat_interval,
				application_context.configuration.missed_heartbeat_limit
			) => left_reason,
		};
		room.remove_client(client_id);

		info!("Client '{}' with id {} has left.", client_name, client_id);
		room.broadcast(ClientLeftBroadcast {
			id: client_id,
			name: client_name,
			reason: left_reason,
		});
	}
}

async fn register_client(
	room: Room,
	message_sender: MessageSender,
	mut message_receiver: MessageReceiver,
) -> Option<(Client, MessageReceiver)> {
	use ReceivedMessage::*;
	let request = match message_receiver.receive().await {
		Finished => {
			error!("Client registration failed. Socket closed prematurely.");
			return None;
		}
		Pong { .. } => {
			error!("Client registration failed. Received Pong instead of register request.");
			return None;
		}
		Request(request) => request,
	};

	let name = if let ClientRequest::Register(RegisterRequest { name }) = request.request {
		name
	} else {
		error!("Client registration failed. Invalid request: {:?}", request);

		let _ = message_sender
			.send_error_message(
				ErrorMessage::builder()
					.error(ErrorMessageType::InvalidOperation)
					.message("Invalid request".to_string())
					.build(),
				Some(request.request_id),
			)
			.await;
		return None;
	};

	let (client, existing_clients) = match room.add_client_and_return_existing(name, message_sender.clone()) {
		Ok(success) => success,
		Err(error) => {
			use RoomError::*;
			let error_response = match error {
				EmptyClientName | ClientNameTooLong => {
					error!("Client registration failed. Tried to register with invalid name.");
					ErrorMessageType::InvalidFormat
				}
				ClientNameAlreadyInUse => {
					error!("Client registration failed. Tried to register with name that is already used.");
					ErrorMessageType::InvalidOperation
				}
				RoomFull => {
					error!("Client registration failed. Room is full.");
					ErrorMessageType::InvalidOperation
				}
			};

			let _ = message_sender
				.send_error_message(
					ErrorMessage::builder()
						.error(error_response)
						.message(error.to_string())
						.build(),
					Some(request.request_id),
				)
				.await;

			return None;
		}
	};

	let clients = existing_clients.into_iter().map(ClientResponse::from).collect();
	let hello_response = SuccessMessage::Hello {
		id: client.id(),
		clients,
		current_medium: room.medium().into(),
	};
	if client.send_success_message(hello_response, request.request_id).await {
		let id = client.id();
		let name = client.name().to_string();

		info!("Registered client: {} {}", id, name);

		room.broadcast(ClientJoinedBroadcast { id, name });
		Some((client, message_receiver))
	} else {
		None
	}
}

pub async fn send_broadcasts(client: Client) {
	loop {
		let broadcast = client.wait_for_broadcast().await;
		if !client.send_broadcast_message(broadcast).await {
			break;
		}
	}
}

pub async fn heartbeat(
	client: Client,
	time_source: &TimeSource,
	mut pong_receiver: mpsc::Receiver<Vec<u8>>,
	heartbeat_interval: std::time::Duration,
	missed_heartbeat_limit: u8,
) -> LeftReason {
	let mut interval = time_source.interval_at(heartbeat_interval, heartbeat_interval);
	let mut missed_heartbeats = 0;

	for count in 0..usize::MAX {
		interval.tick().await;

		if !client.send_ping(count.to_ne_bytes().as_ref().into()).await {
			return LeftReason::Closed;
		}

		let receive_pong = async {
			while let Some(payload) = pong_receiver.next().await {
				let payload = match <[u8; std::mem::size_of::<usize>()]>::try_from(payload.as_ref()) {
					Ok(payload) => payload,
					Err(_) => return Err(()),
				};

				let received_count = usize::from_ne_bytes(payload);
				if received_count == count {
					return Ok(());
				}
			}
			Err(())
		};
		if time_source.timeout(heartbeat_interval, receive_pong).await.is_err() {
			missed_heartbeats += 1;
			if missed_heartbeats >= missed_heartbeat_limit {
				break;
			}
		} else {
			missed_heartbeats = 0;
		}
	}

	LeftReason::Timeout
}

const QUOTA: Quota = Quota::per_second(nonzero!(1u32)).allow_burst(nonzero!(10u32));

async fn handle_messages(
	room: &Room,
	client: Client,
	mut message_receiver: MessageReceiver,
	mut pong_sender: mpsc::Sender<Vec<u8>>,
) {
	let rate_limiter = RateLimiter::direct(QUOTA);
	loop {
		let message = match message_receiver.receive().await {
			ReceivedMessage::Request(message) => message,
			ReceivedMessage::Pong { payload } => {
				if pong_sender.send(payload).await.is_err() {
					break;
				}
				continue;
			}
			ReceivedMessage::Finished => break,
		};

		// rate limit after receiving a message so we don't apply it to receiving pong messages
		rate_limiter.until_ready().await;

		debug!(
			"Received {} message from '{}' (#{})",
			message.request.kind(),
			client.name(),
			Into::<PortableUnsignedInteger>::into(client.id()),
		);

		match handle_request(room, &client, message.request) {
			Ok(success_message) => client.send_success_message(success_message, message.request_id).await,
			Err(error_message) => client.send_error_message(error_message, Some(message.request_id)).await,
		};
	}
}

fn handle_request(room: &Room, client: &Client, request: ClientRequest) -> Result<SuccessMessage, ErrorMessage> {
	use ClientRequest::*;
	match request {
		Chat(chat_request) => handle_chat_request(room, client, chat_request),
		Register { .. } => handle_register_request(client),
		GetReferenceTime => Ok(handle_get_reference_time_request(room)),
		InsertMedium(insert_medium_request) => handle_insert_medium_request(room, client, insert_medium_request),
		Play(play_request) => handle_play_request(room, client, play_request),
		Pause(pause_request) => handle_pause_request(room, client, pause_request),
	}
}

fn handle_chat_request(
	room: &Room,
	client: &Client,
	ChatRequest { message }: ChatRequest,
) -> Result<SuccessMessage, ErrorMessage> {
	if message.trim().is_empty() {
		return Err(ErrorMessage::builder()
			.error(ErrorMessageType::EmptyChatMessage)
			.message("Chat messages must not be empty!".to_string())
			.build());
	}
	room.send_chat_message(client, message);
	Ok(SuccessMessage::Success)
}

fn handle_register_request(client: &Client) -> Result<SuccessMessage, ErrorMessage> {
	error!(
		"Client: {} tried to register even though it is already registered.",
		client.id()
	);
	Err(ErrorMessage::builder()
		.error(ErrorMessageType::InvalidOperation)
		.message("Already registered".to_string())
		.build())
}

fn handle_get_reference_time_request(room: &Room) -> SuccessMessage {
	let reference_time = room.current_reference_time();
	SuccessMessage::ReferenceTime {
		milliseconds: u64::try_from(reference_time.as_millis()).unwrap(),
	}
}

fn handle_insert_medium_request(
	room: &Room,
	client: &Client,
	InsertMediumRequest {
		previous_version,
		medium: medium_request,
	}: InsertMediumRequest,
) -> Result<SuccessMessage, ErrorMessage> {
	let medium = Medium::try_from(medium_request)?;
	let versioned_medium = match room.insert_medium(medium, previous_version) {
		Some(versioned_medium) => versioned_medium,
		None => {
			return Err(ErrorMessage {
				error: ErrorMessageType::IncorrectMediumVersion,
				message: format!(
					"Medium version is incorrect. Request had {} but current version is {}.",
					previous_version,
					room.medium().version
				),
			})
		}
	};

	room.broadcast(MediumStateChangedBroadcast {
		changed_by_name: client.name().to_string(),
		changed_by_id: client.id(),
		medium: VersionedMediumBroadcast::new(versioned_medium, false),
	});

	Ok(SuccessMessage::Success)
}

fn handle_play_request(
	room: &Room,
	client: &Client,
	PlayRequest {
		previous_version,
		skipped,
		start_time_in_milliseconds,
	}: PlayRequest,
) -> Result<SuccessMessage, ErrorMessage> {
	let versioned_medium = match room.play_medium(Duration::milliseconds(start_time_in_milliseconds), previous_version)
	{
		None => {
			return Err(ErrorMessage {
				error: ErrorMessageType::IncorrectMediumVersion,
				message: format!(
					"Medium version is incorrect. Request had {} but current version is {}.",
					previous_version,
					room.medium().version
				),
			})
		}
		Some(versioned_medium) => versioned_medium,
	};
	room.broadcast(MediumStateChangedBroadcast {
		changed_by_name: client.name().to_string(),
		changed_by_id: client.id(),
		medium: VersionedMediumBroadcast::new(versioned_medium, skipped),
	});
	Ok(SuccessMessage::Success)
}

fn handle_pause_request(
	room: &Room,
	client: &Client,
	PauseRequest {
		previous_version,
		skipped,
		position_in_milliseconds,
	}: PauseRequest,
) -> Result<SuccessMessage, ErrorMessage> {
	let versioned_medium = match room.pause_medium(
		Duration::milliseconds(
			i64::try_from(position_in_milliseconds.max(0).min(u64::try_from(i64::MAX).unwrap())).unwrap(),
		),
		previous_version,
	) {
		None => {
			return Err(ErrorMessage {
				error: ErrorMessageType::IncorrectMediumVersion,
				message: format!(
					"Medium version is incorrect. Request had {} but current version is {}.",
					previous_version,
					room.medium().version
				),
			})
		}
		Some(versioned_medium) => versioned_medium,
	};
	room.broadcast(MediumStateChangedBroadcast {
		changed_by_name: client.name().to_string(),
		changed_by_id: client.id(),
		medium: VersionedMediumBroadcast::new(versioned_medium, skipped),
	});
	Ok(SuccessMessage::Success)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::lifecycle::{handle_messages, handle_request, register_client};
	use crate::message::client_request::{MediumRequest, PauseRequest};
	use crate::message::outgoing::broadcast_message::{BroadcastMessage, ChatBroadcast, MediumBroadcast};
	use crate::message::outgoing::error_message::ErrorMessageType;
	use crate::message::outgoing::success_message::{MediumResponse, PlaybackStateResponse, VersionedMediumResponse};
	use crate::room::client_id::ClientId;
	use crate::room::medium::fixed_length::FixedLengthMedium;
	use crate::room::medium::VersionedMedium;
	use crate::utils::fake_message_sender::FakeMessageSender;
	use crate::utils::test_client::WebsocketTestClient;
	use tokio::time::sleep;

	#[tokio::test]
	async fn the_client_should_get_access_to_the_server_reference_time() {
		const TEST_DELAY: std::time::Duration = std::time::Duration::from_millis(2);

		let (message_sender, _message_receiver, _test_client) = WebsocketTestClient::new();
		let room = Room::new(10);
		let (client, _) = room
			.add_client_and_return_existing("Alice".to_string(), message_sender)
			.expect("Did not get client handle!");

		sleep(TEST_DELAY).await; // ensure that some time has passed
		let response = handle_request(&room, &client, ClientRequest::GetReferenceTime)
			.expect("Failed to get reference time message");

		match response {
			SuccessMessage::ReferenceTime { milliseconds } => {
				assert!(
					(milliseconds >= u64::try_from(TEST_DELAY.as_millis()).unwrap()) && (milliseconds < 1000),
					"milliseconds = {}",
					milliseconds
				);
			}
			_ => panic!("Invalid response"),
		};
	}

	#[tokio::test]
	async fn the_client_should_get_an_error_for_empty_chat_messages() {
		let room = Room::new(1);
		let (client, mut test_client) = WebsocketTestClient::in_room("Alice", &room).await;

		let empty_chat_request = ChatRequest {
			message: " \t".to_string(),
		};
		let non_empty_chat_request = ChatRequest {
			message: "Hi!".to_string(),
		};
		let error =
			handle_request(&room, &client, empty_chat_request.into()).expect_err("Accepted empty chat message.");
		handle_request(&room, &client, non_empty_chat_request.clone().into())
			.expect("Failed to send proper chat message");

		assert_eq!(
			error,
			ErrorMessage::builder()
				.error(ErrorMessageType::EmptyChatMessage)
				.message("Chat messages must not be empty!".to_string())
				.build()
		);

		// ensure we don't see the empty chat message
		let received_message = test_client.receive_broadcast_message().await;
		assert_eq!(
			received_message,
			BroadcastMessage::Chat(ChatBroadcast {
				sender_id: client.id(),
				sender_name: client.name().to_string(),
				message: non_empty_chat_request.message,
				counter: 0
			})
		);
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_insert_a_medium() {
		let room = Room::new(2);
		let (alice, mut alice_test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (_bob, mut bob_test_client) = WebsocketTestClient::in_room("Bob", &room).await;

		let medium = FixedLengthMedium::new("Metropolis".to_string(), Duration::minutes(153));
		let response = handle_request(
			&room,
			&alice,
			InsertMediumRequest {
				medium: Medium::from(medium.clone()).into(),
				previous_version: 0,
			}
			.into(),
		)
		.expect("Failed to get successful response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = MediumStateChangedBroadcast {
			changed_by_name: alice.name().to_string(),
			changed_by_id: alice.id(),
			medium: VersionedMediumBroadcast::new(
				VersionedMedium {
					medium: medium.into(),
					version: 1,
				},
				false,
			),
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_not_be_able_to_insert_a_too_large_medium() {
		let (alice_message_sender, _message_receiver, _alice_test_client) = WebsocketTestClient::new();

		let room = Room::new(2);
		let (alice, _) = room
			.add_client_and_return_existing("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");

		let request = InsertMediumRequest {
			medium: MediumRequest::FixedLength {
				name: "Metropolis".to_string(),
				length_in_milliseconds: u64::try_from(Duration::days(400).num_milliseconds()).unwrap(),
			},
			previous_version: 0,
		};
		let response = handle_request(&room, &alice, request.into()).expect_err("Failed to ger error response");

		assert_eq!(
			response,
			ErrorMessage::builder()
				.error(ErrorMessageType::InvalidFormat)
				.message("Length of a medium must not be larger than one year.".to_string())
				.build()
		);
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_play_the_inserted_medium() {
		let room = Room::new(2);
		let (alice, mut alice_test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (_bob, mut bob_test_client) = WebsocketTestClient::in_room("Bob", &room).await;

		let medium = FixedLengthMedium::new("Metropolis".to_string(), Duration::minutes(153));
		let inserted_medium = room.insert_medium(medium.clone(), 0).expect("Failed to insert medium");

		let response = handle_request(
			&room,
			&alice,
			PlayRequest {
				previous_version: inserted_medium.version,
				skipped: true,
				start_time_in_milliseconds: -1024,
			}
			.into(),
		)
		.expect("Failed to get success response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = MediumStateChangedBroadcast {
			changed_by_name: alice.name().to_string(),
			changed_by_id: alice.id(),
			medium: VersionedMediumBroadcast {
				medium: MediumBroadcast::FixedLength {
					name: medium.name,
					length_in_milliseconds: u64::try_from(medium.length.num_milliseconds()).unwrap(),
					playback_skipped: true,
					playback_state: PlaybackStateResponse::Playing {
						start_time_in_milliseconds: -1024,
					},
				},
				version: 2,
			},
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_pause_the_inserted_medium() {
		let room = Room::new(2);
		let (_alice, mut alice_test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (bob, mut bob_test_client) = WebsocketTestClient::in_room("Bob", &room).await;

		let medium = FixedLengthMedium::new("Metropolis".to_string(), Duration::minutes(153));
		let inserted_medium = room.insert_medium(medium.clone(), 0).expect("Failed to insert medium");
		let played_medium = room
			.play_medium(Duration::milliseconds(-1024), inserted_medium.version)
			.expect("Failed to play medium.");

		let response = handle_request(
			&room,
			&bob,
			PauseRequest {
				previous_version: played_medium.version,
				skipped: false,
				position_in_milliseconds: 1027,
			}
			.into(),
		)
		.expect("Failed to get success response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = MediumStateChangedBroadcast {
			changed_by_name: bob.name().to_string(),
			changed_by_id: bob.id(),
			medium: VersionedMediumBroadcast {
				medium: MediumBroadcast::FixedLength {
					name: medium.name,
					length_in_milliseconds: u64::try_from(medium.length.num_milliseconds()).unwrap(),
					playback_skipped: false,
					playback_state: PlaybackStateResponse::Paused {
						position_in_milliseconds: 1027,
					},
				},
				version: 3,
			},
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_skip_in_paused_mode() {
		let room = Room::new(2);
		let (_alice, mut alice_test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (bob, mut bob_test_client) = WebsocketTestClient::in_room("Bob", &room).await;

		let medium = FixedLengthMedium::new("Metropolis".to_string(), Duration::minutes(153));
		let inserted_medium = room.insert_medium(medium.clone(), 0).expect("Failed to insert medium");

		let response = handle_request(
			&room,
			&bob,
			PauseRequest {
				previous_version: inserted_medium.version,
				skipped: true,
				position_in_milliseconds: 1000,
			}
			.into(),
		)
		.expect("Failed to get success response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = MediumStateChangedBroadcast {
			changed_by_name: bob.name().to_string(),
			changed_by_id: bob.id(),
			medium: VersionedMediumBroadcast {
				medium: MediumBroadcast::FixedLength {
					name: medium.name,
					length_in_milliseconds: u64::try_from(medium.length.num_milliseconds()).unwrap(),
					playback_skipped: true,
					playback_state: PlaybackStateResponse::Paused {
						position_in_milliseconds: 1000,
					},
				},
				version: 2,
			},
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_not_be_able_to_play_with_incorrect_version() {
		let (alice_message_sender, _message_receiver, _alice_test_client) = WebsocketTestClient::new();

		let room = Room::new(1);
		let (alice, _) = room
			.add_client_and_return_existing("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");

		let medium = FixedLengthMedium::new("Metropolis".to_string(), Duration::minutes(153));
		let inserted_medium = room.insert_medium(medium, 0).expect("Failed to insert medium");

		let response = handle_request(
			&room,
			&alice,
			PlayRequest {
				previous_version: inserted_medium.version + 1,
				skipped: true,
				start_time_in_milliseconds: 0,
			}
			.into(),
		)
		.expect_err("Failed to get error response");
		assert_eq!(
			response,
			ErrorMessage::builder()
				.error(ErrorMessageType::IncorrectMediumVersion)
				.message("Medium version is incorrect. Request had 2 but current version is 1.".to_string())
				.build()
		);
	}

	#[tokio::test]
	async fn the_client_should_not_be_able_to_pause_with_incorrect_version() {
		let (alice_message_sender, _message_receiver, _alice_test_client) = WebsocketTestClient::new();

		let room = Room::new(1);
		let (alice, _) = room
			.add_client_and_return_existing("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");

		let medium = FixedLengthMedium::new("Metropolis".to_string(), Duration::minutes(153));
		let inserted_medium = room.insert_medium(medium, 0).expect("Failed to insert medium");

		let response = handle_request(
			&room,
			&alice,
			PauseRequest {
				previous_version: inserted_medium.version + 1,
				skipped: true,
				position_in_milliseconds: 0,
			}
			.into(),
		)
		.expect_err("Failed to get error response");
		assert_eq!(
			response,
			ErrorMessage::builder()
				.error(ErrorMessageType::IncorrectMediumVersion)
				.message("Medium version is incorrect. Request had 2 but current version is 1.".to_string())
				.build()
		);
	}

	#[tokio::test]
	async fn the_client_should_not_be_able_to_insert_medium_with_incorrect_version() {
		let (alice_message_sender, _message_receiver, _alice_test_client) = WebsocketTestClient::new();

		let room = Room::new(1);
		let (alice, _) = room
			.add_client_and_return_existing("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");

		let response = handle_request(
			&room,
			&alice,
			InsertMediumRequest {
				previous_version: 1,
				medium: MediumRequest::Empty,
			}
			.into(),
		)
		.expect_err("Failed to get error response");
		assert_eq!(
			response,
			ErrorMessage::builder()
				.error(ErrorMessageType::IncorrectMediumVersion)
				.message("Medium version is incorrect. Request had 1 but current version is 0.".to_string())
				.build()
		);
	}

	#[tokio::test]
	async fn should_not_allow_registering_client_twice() {
		let (message_sender, message_receiver, test_client) = WebsocketTestClient::new();
		let room = Room::new(10);

		let (client_handle, message_receiver, mut test_client) =
			register_test_client("Anorak", room.clone(), message_sender, message_receiver, test_client).await;

		// run server message handler
		tokio::spawn({
			async move {
				let room = &room;
				let (pong_sender, _pong_receiver) = mpsc::channel(0);
				handle_messages(room, client_handle, message_receiver, pong_sender).await;
			}
		});

		let register_message = RegisterRequest {
			name: "Parcival".to_string(),
		};

		let request_id = test_client.send_request(register_message).await;
		let error = test_client.receive_error_message(Some(request_id)).await;
		assert_eq!(
			error,
			ErrorMessage::builder()
				.error(ErrorMessageType::InvalidOperation)
				.message("Already registered".to_string())
				.build()
		);
	}

	#[tokio::test]
	async fn should_not_register_clients_with_blank_name() {
		let (message_sender, message_receiver, mut test_client) = WebsocketTestClient::new();
		let room = Room::new(10);
		let register_request = RegisterRequest { name: "	 ".to_string() };

		let request_id = test_client.send_request(register_request).await;
		register_client(room, message_sender, message_receiver).await;
		let response = test_client.receive_error_message(Some(request_id)).await;

		assert_eq!(
			ErrorMessage::builder()
				.error(ErrorMessageType::InvalidFormat)
				.message("Name was empty or whitespace-only.".to_string())
				.build(),
			response
		);
	}

	#[tokio::test]
	async fn should_not_register_clients_with_already_registered_name() {
		let room = Room::new(10);

		// "Ferris" is already a registered client
		let fake_message_sender = FakeMessageSender::default().into();
		room.add_client_and_return_existing("Ferris".to_string(), fake_message_sender)
			.expect("Could not register 'Ferris'!");

		// And I register another client with the same name
		let (message_sender, message_receiver, mut test_client) = WebsocketTestClient::new();
		let register_request = RegisterRequest {
			name: "Ferris".to_string(),
		};

		let request_id = test_client.send_request(register_request).await;
		register_client(room, message_sender, message_receiver).await;
		let response = test_client.receive_error_message(Some(request_id)).await;

		// Then I expect an error
		assert_eq!(
			ErrorMessage::builder()
				.error(ErrorMessageType::InvalidOperation)
				.message("Client name is already in use.".to_string())
				.build(),
			response
		);
	}

	#[tokio::test]
	async fn should_not_register_clients_if_room_is_full() {
		let room = Room::new(1);
		{
			let message_sender = MessageSender::from(FakeMessageSender::default());
			room.add_client_and_return_existing("Fake".to_string(), message_sender)
				.unwrap();
		}

		let (message_sender, message_receiver, mut test_client) = WebsocketTestClient::new();
		let register_request = RegisterRequest {
			name: "second".to_string(),
		};

		let request_id = test_client.send_request(register_request).await;
		register_client(room, message_sender, message_receiver).await;
		let response = test_client.receive_error_message(Some(request_id)).await;

		assert_eq!(
			ErrorMessage::builder()
				.error(ErrorMessageType::InvalidOperation)
				.message("Can't join, room is already full.".to_string())
				.build(),
			response
		);
	}

	#[tokio::test]
	async fn should_get_currently_playing_medium_on_hello_response() {
		let room = Room::new(1);
		let video_name = "Short Circuit".to_string();
		let video_length = Duration::minutes(98);
		let short_circuit = FixedLengthMedium::new(video_name.clone(), video_length);
		let inserted_medium = room.insert_medium(short_circuit, 0).expect("Failed to insert medium");
		room.play_medium(Duration::milliseconds(0), inserted_medium.version)
			.expect("Must successfully start playing");

		let (message_sender, message_receiver, mut test_client) = WebsocketTestClient::new();
		let register_request = RegisterRequest {
			name: "Johnny 5".to_string(),
		};

		let request_id = test_client.send_request(register_request).await;
		register_client(room, message_sender, message_receiver).await;
		let response = test_client.receive_success_message(request_id).await;

		assert_eq!(
			SuccessMessage::Hello {
				id: ClientId::from(0),
				clients: vec![],
				current_medium: VersionedMediumResponse {
					medium: MediumResponse::FixedLength {
						name: video_name,
						length_in_milliseconds: u64::try_from(video_length.num_milliseconds()).unwrap(),
						playback_state: PlaybackStateResponse::Playing {
							start_time_in_milliseconds: 0,
						}
					},
					version: 2
				}
			},
			response
		);
	}

	#[tokio::test]
	async fn should_list_other_clients_when_joining_a_room() {
		let room = Room::new(2);
		let fake_message_sender = FakeMessageSender::default();
		let (stephanie, _) = room
			.add_client_and_return_existing("Stephanie".to_string(), fake_message_sender.into())
			.unwrap();

		let (message_sender, message_receiver, mut test_client) = WebsocketTestClient::new();
		let register_request = RegisterRequest {
			name: "Johnny 5".to_string(),
		};

		let request_id = test_client.send_request(register_request).await;
		register_client(room, message_sender, message_receiver).await;
		let response = test_client.receive_success_message(request_id).await;

		assert_eq!(
			SuccessMessage::Hello {
				id: ClientId::from(1),
				clients: vec![ClientResponse {
					id: stephanie.id(),
					name: stephanie.name().to_string(),
				}],
				current_medium: VersionedMedium::default().into(),
			},
			response
		);
	}

	#[tokio::test]
	async fn should_not_list_any_clients_when_joining_an_empty_room() {
		let room = Room::new(1);
		let (message_sender, message_receiver, mut test_client) = WebsocketTestClient::new();
		let register_request = RegisterRequest {
			name: "Johnny 5".to_string(),
		};

		let request_id = test_client.send_request(register_request).await;
		register_client(room, message_sender, message_receiver).await;
		let response = test_client.receive_success_message(request_id).await;

		assert_eq!(
			SuccessMessage::Hello {
				id: ClientId::from(0),
				clients: vec![],
				current_medium: VersionedMedium::default().into(),
			},
			response
		);
	}

	#[tokio::test]
	async fn should_get_currently_paused_medium_on_hello_response() {
		let room = Room::new(1);
		let video_name = "Short Circuit".to_string();
		let video_length = Duration::minutes(98);
		let short_circuit = FixedLengthMedium::new(video_name.clone(), video_length);
		room.insert_medium(short_circuit, 0).unwrap();

		let (message_sender, message_receiver, mut test_client) = WebsocketTestClient::new();
		let register_request = RegisterRequest {
			name: "Johnny 5".to_string(),
		};

		let request_id = test_client.send_request(register_request).await;
		register_client(room, message_sender, message_receiver).await;
		let response = test_client.receive_success_message(request_id).await;

		assert_eq!(
			SuccessMessage::Hello {
				id: ClientId::from(0),
				clients: vec![],
				current_medium: VersionedMediumResponse {
					medium: MediumResponse::FixedLength {
						name: video_name,
						length_in_milliseconds: u64::try_from(video_length.num_milliseconds()).unwrap(),
						playback_state: PlaybackStateResponse::Paused {
							position_in_milliseconds: 0
						}
					},
					version: 1
				}
			},
			response
		);
	}

	#[tokio::test]
	async fn should_send_heartbeats_with_test_time_source() {
		let room = Room::new(1);
		let time_source = TimeSource::test();
		let (client, mut test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (mut pong_sender, pong_receiver) = mpsc::channel(0);

		let heartbeat_interval = std::time::Duration::from_millis(1);
		let time_source_for_heartbeat = time_source.clone();

		tokio::spawn(async move {
			let left_reason = heartbeat(client, &time_source_for_heartbeat, pong_receiver, heartbeat_interval, 0).await;
			assert_eq!(left_reason, LeftReason::Closed); // NOTE: This line will most likely never run
		});

		time_source.wait_for_time_request().await;
		const ITERATIONS: u32 = MISSED_HEARTBEAT_LIMIT + 1;
		for _ in 0..ITERATIONS {
			time_source.advance_time(heartbeat_interval);
			let payload = test_client.receive_ping().await;
			pong_sender.send(payload).await.unwrap();
		}
	}

	#[tokio::test]
	async fn should_send_heartbeats_with_real_time_source() {
		let room = Room::new(1);
		let time_source = TimeSource::default();
		let (client, mut test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (mut pong_sender, pong_receiver) = mpsc::channel(0);

		let heartbeat_interval = std::time::Duration::from_millis(1);
		let time_source_for_heartbeat = time_source.clone();

		tokio::spawn(async move {
			let left_reason = heartbeat(client, &time_source_for_heartbeat, pong_receiver, heartbeat_interval, 0).await;
			assert_eq!(left_reason, LeftReason::Closed); // NOTE: This line will most likely never run
		});

		let payload = test_client.receive_ping().await;
		pong_sender.send(payload).await.unwrap();
	}

	#[tokio::test]
	async fn should_stop_after_missed_heartbeat_limit_with_test_time_source() {
		let room = Room::new(1);
		let time_source = TimeSource::test();
		let (client, _test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (_pong_sender, pong_receiver) = mpsc::channel(0);

		let heartbeat_interval = std::time::Duration::from_millis(1);
		let missed_heartbeat_limit = 1;

		// task for advancing test time
		let time_source_for_test = time_source.clone();
		tokio::spawn(async move {
			let time_source = time_source_for_test;

			time_source.wait_for_time_request().await;
			time_source.advance_time(MISSED_HEARTBEAT_LIMIT * heartbeat_interval);

			for _ in 0..MISSED_HEARTBEAT_LIMIT {
				time_source.wait_for_time_request().await;
				time_source.advance_time(heartbeat_interval);
			}
		});

		let left_reason = heartbeat(
			client,
			&time_source,
			pong_receiver,
			heartbeat_interval,
			missed_heartbeat_limit,
		)
		.await;
		assert_eq!(left_reason, LeftReason::Timeout);
	}

	#[tokio::test]
	async fn should_stop_after_missed_heartbeats_with_real_time_source() {
		let room = Room::new(1);
		let time_source = TimeSource::default();
		let (client, _test_client) = WebsocketTestClient::in_room("Alice", &room).await;
		let (_pong_sender, pong_receiver) = mpsc::channel(0);

		let heartbeat_interval = std::time::Duration::from_millis(1);
		let missed_heartbeat_limit = 1;

		let left_reason = heartbeat(
			client,
			&time_source,
			pong_receiver,
			heartbeat_interval,
			missed_heartbeat_limit,
		)
		.await;
		assert_eq!(left_reason, LeftReason::Timeout);
	}

	async fn register_test_client(
		name: &'static str,
		room: Room,
		message_sender: MessageSender,
		message_receiver: MessageReceiver,
		mut test_client: WebsocketTestClient,
	) -> (Client, MessageReceiver, WebsocketTestClient) {
		let register_request = RegisterRequest { name: name.into() };

		let request_id = test_client.send_request(register_request).await;

		// run server code required for client registration
		let (client, message_receiver) = register_client(room.clone(), message_sender, message_receiver)
			.await
			.unwrap();

		let response = test_client.receive_success_message(request_id).await;

		let id = if let SuccessMessage::Hello { id, .. } = response {
			id
		} else {
			panic!("Expected Hello-Response, got '{:?}'", response);
		};
		assert_eq!(client.id(), id);

		let joined_response = client.wait_for_broadcast().await;
		assert!(matches!(
			joined_response,
			BroadcastMessage::ClientJoined(ClientJoinedBroadcast { id: _, name: _ })
		));
		(client, message_receiver, test_client)
	}
}
