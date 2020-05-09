use crate::connection::receiver::MessageReceiver;
use crate::connection::sender::MessageSender;
use crate::message::client_request::{
	ChatRequest, ClientRequest, InsertMediumRequest, PauseRequest, PlayRequest, RegisterRequest,
};
use crate::message::outgoing::broadcast_message::{
	ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast, MediumInsertedBroadcast, PlaybackStateChangedBroadcast,
};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::outgoing::success_message::{ClientResponse, MediumResponse, SuccessMessage};
use crate::room::client::Client;
use crate::room::error::RoomError;
use crate::room::state::medium::fixed_length::FixedLengthMedium;
use crate::room::state::medium::SomeMedium;
use crate::room::Room;
use chrono::Duration;
use log::{debug, error, info};

pub async fn run_client(room: Room, message_sender: MessageSender, message_receiver: MessageReceiver) {
	if let Some((client, message_receiver)) = register_client(room.clone(), message_sender, message_receiver).await {
		let client_id = client.id();
		handle_messages(&room, client, message_receiver).await;
		room.remove_client(client_id);
	}
}

async fn register_client(
	room: Room,
	message_sender: MessageSender,
	mut message_receiver: MessageReceiver,
) -> Option<(Client, MessageReceiver)> {
	let request = match message_receiver.receive().await {
		None => {
			error!("Client registration failed. Socket closed prematurely.");
			return None;
		}
		Some(request) => request,
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

	let client = match room.add_client(name, message_sender.clone()) {
		Ok(client) => client,
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

	let clients = room
		.clients()
		.filter(|(id, _)| *id != client.id())
		.map(ClientResponse::from)
		.collect();
	let hello_response = SuccessMessage::Hello {
		id: client.id(),
		clients,
		current_medium: room.medium().as_ref().map(MediumResponse::from),
	};
	if client.send_success_message(hello_response, request.request_id).await {
		let id = client.id();
		let name = client.name().to_string();

		info!("Registered client: {} {}", id, name);

		room.broadcast(ClientJoinedBroadcast { id, name }).await;
		Some((client, message_receiver))
	} else {
		None
	}
}

async fn handle_messages(room: &Room, client: Client, mut message_receiver: MessageReceiver) {
	loop {
		let message = match message_receiver.receive().await {
			Some(message) => message,
			None => break, // connection has been closed
		};
		debug!(
			"Received {:?} message from {}",
			std::mem::discriminant(&message),
			client.id(),
		);

		match handle_request(room, &client, message.request).await {
			Ok(success_message) => client.send_success_message(success_message, message.request_id).await,
			Err(error_message) => client.send_error_message(error_message, Some(message.request_id)).await,
		};
	}

	let id = client.id();
	let name = client.name().to_string();
	info!("Client '{}' with id {} has left.", name, id);
	room.broadcast(ClientLeftBroadcast { id, name }).await;
}

async fn handle_request(room: &Room, client: &Client, request: ClientRequest) -> Result<SuccessMessage, ErrorMessage> {
	use ClientRequest::*;
	match request {
		Chat(ChatRequest { message }) => {
			room.broadcast(ChatBroadcast {
				sender_id: client.id(),
				sender_name: client.name().to_string(),
				message,
			})
			.await;
			Ok(SuccessMessage::Success)
		}
		Register { .. } => {
			error!(
				"Client: {} tried to register even though it is already registered.",
				client.id()
			);
			Err(ErrorMessage::builder()
				.error(ErrorMessageType::InvalidOperation)
				.message("Already registered".to_string())
				.build()
				.into())
		}
		GetReferenceTime => {
			let reference_time = room.current_reference_time();
			Ok(SuccessMessage::ReferenceTime {
				milliseconds: reference_time.as_millis() as u64,
			})
		}
		InsertMedium(InsertMediumRequest {
			name,
			length_in_milliseconds,
		}) => {
			if length_in_milliseconds > (Duration::days(365).num_milliseconds() as u64) {
				Err(ErrorMessage::builder()
					.error(ErrorMessageType::InvalidFormat)
					.message("Length of a medium must not be larger than one year.".to_string())
					.build()
					.into())
			} else {
				room.insert_medium(SomeMedium::FixedLength(FixedLengthMedium::new(
					name.clone(),
					Duration::milliseconds(length_in_milliseconds as i64),
				)));

				room.broadcast(MediumInsertedBroadcast {
					inserted_by_name: client.name().to_string(),
					inserted_by_id: client.id(),
					name,
					length_in_milliseconds,
				})
				.await;
				Ok(SuccessMessage::Success)
			}
		}
		Play(PlayRequest {
			skipped,
			start_time_in_milliseconds,
		}) => match room.play_medium(Duration::milliseconds(start_time_in_milliseconds)) {
			None => Err(ErrorMessage::builder()
				.error(ErrorMessageType::NoMedium)
				.message("Room has no medium.".to_string())
				.build()),
			Some(playback_state) => {
				room.broadcast(PlaybackStateChangedBroadcast {
					changed_by_name: client.name().to_string(),
					changed_by_id: client.id(),
					skipped,
					playback_state: playback_state.into(),
				})
				.await;
				Ok(SuccessMessage::Success)
			}
		},
		Pause(PauseRequest {
			skipped,
			position_in_milliseconds,
		}) => match room.pause_medium(Duration::milliseconds(
			position_in_milliseconds.max(0).min(std::i64::MAX as u64) as i64,
		)) {
			None => Err(ErrorMessage::builder()
				.error(ErrorMessageType::NoMedium)
				.message("Room has no medium.".to_string())
				.build()),
			Some(playback_state) => {
				room.broadcast(PlaybackStateChangedBroadcast {
					changed_by_name: client.name().to_string(),
					changed_by_id: client.id(),
					skipped,
					playback_state: playback_state.into(),
				})
				.await;
				Ok(SuccessMessage::Success)
			}
		},
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::lifecycle::{handle_messages, handle_request, register_client};
	use crate::message::client_request::PauseRequest;
	use crate::message::outgoing::broadcast_message::BroadcastMessage;
	use crate::message::outgoing::error_message::ErrorMessageType;
	use crate::message::outgoing::success_message::PlaybackStateResponse;
	use crate::room::client_id::ClientId;
	use crate::utils::fake_connection::FakeClientConnection;
	use crate::utils::test_client::WebsocketTestClient;
	use tokio::time::delay_for;

	#[tokio::test]
	async fn the_client_should_get_access_to_the_server_reference_time() {
		const TEST_DELAY: std::time::Duration = std::time::Duration::from_millis(2);

		let (message_sender, _message_receiver, _test_client) = WebsocketTestClient::new();
		let room = Room::new(10);
		let client_handle = room
			.add_client("Alice".to_string(), message_sender)
			.expect("Did not get client handle!");

		delay_for(TEST_DELAY).await; // ensure that some time has passed
		let response = handle_request(&room, &client_handle, ClientRequest::GetReferenceTime)
			.await
			.expect("Failed to get reference time message");

		match response {
			SuccessMessage::ReferenceTime { milliseconds } => {
				assert!(
					(milliseconds >= TEST_DELAY.as_millis() as u64) && (milliseconds < 1000),
					"milliseconds = {}",
					milliseconds
				);
			}
			_ => panic!("Invalid response"),
		};
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_insert_a_medium() {
		let (alice_message_sender, _message_receiver, mut alice_test_client) = WebsocketTestClient::new();
		let (bob_message_sender, _message_receiver, mut bob_test_client) = WebsocketTestClient::new();

		let room = Room::new(2);
		let alice = room
			.add_client("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");
		room.add_client("Bob".to_string(), bob_message_sender)
			.expect("Did not get client handle!");

		let response = handle_request(
			&room,
			&alice,
			InsertMediumRequest {
				name: "Metropolis".to_string(),
				length_in_milliseconds: 153 * 60 * 1000,
			}
			.into(),
		)
		.await
		.expect("Failed to get successful response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = MediumInsertedBroadcast {
			inserted_by_name: alice.name().to_string(),
			inserted_by_id: alice.id(),
			name: "Metropolis".to_string(),
			length_in_milliseconds: 153 * 60 * 1000,
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_not_be_able_to_insert_a_too_large_medium() {
		let (alice_message_sender, _message_receiver, _alice_test_client) = WebsocketTestClient::new();

		let room = Room::new(2);
		let alice = room
			.add_client("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");

		let response = handle_request(
			&room,
			&alice,
			InsertMediumRequest {
				name: "Metropolis".to_string(),
				length_in_milliseconds: Duration::days(400).num_milliseconds() as u64,
			}
			.into(),
		)
		.await
		.expect_err("Failed to ger error response");

		assert_eq!(
			response,
			ErrorMessage::builder()
				.error(ErrorMessageType::InvalidFormat)
				.message("Length of a medium must not be larger than one year.".to_string())
				.build()
		)
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_play_the_inserted_medium() {
		let (alice_message_sender, _message_receiver, mut alice_test_client) = WebsocketTestClient::new();
		let (bob_message_sender, _message_receiver, mut bob_test_client) = WebsocketTestClient::new();

		let room = Room::new(2);
		let alice = room
			.add_client("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");
		room.add_client("Bob".to_string(), bob_message_sender)
			.expect("Did not get client handle!");
		room.insert_medium(SomeMedium::FixedLength(FixedLengthMedium::new(
			"Metropolis".to_string(),
			Duration::minutes(153),
		)));

		let response = handle_request(
			&room,
			&alice,
			PlayRequest {
				skipped: true,
				start_time_in_milliseconds: -1024,
			}
			.into(),
		)
		.await
		.expect("Failed to get success response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = PlaybackStateChangedBroadcast {
			changed_by_name: alice.name().to_string(),
			changed_by_id: alice.id(),
			skipped: true,
			playback_state: PlaybackStateResponse::Playing {
				start_time_in_milliseconds: -1024,
			},
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_not_be_able_to_play_something_without_medium() {
		let (alice_message_sender, _message_receiver, _alice_test_client) = WebsocketTestClient::new();

		let room = Room::new(1);
		let alice = room
			.add_client("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");

		let response = handle_request(
			&room,
			&alice,
			PlayRequest {
				skipped: true,
				start_time_in_milliseconds: -1024,
			}
			.into(),
		)
		.await
		.expect_err("Failed to get error response");

		assert_eq!(
			response,
			ErrorMessage::builder()
				.error(ErrorMessageType::NoMedium)
				.message("Room has no medium.".to_string())
				.build()
		);
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_pause_the_inserted_medium() {
		let (alice_message_sender, _message_receiver, mut alice_test_client) = WebsocketTestClient::new();
		let (bob_message_sender, _message_receiver, mut bob_test_client) = WebsocketTestClient::new();

		let room = Room::new(2);
		room.add_client("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");
		let bob = room
			.add_client("Bob".to_string(), bob_message_sender)
			.expect("Did not get client handle!");
		room.insert_medium(SomeMedium::FixedLength(FixedLengthMedium::new(
			"Metropolis".to_string(),
			Duration::minutes(153),
		)));
		room.play_medium(Duration::milliseconds(-1024));

		let response = handle_request(
			&room,
			&bob,
			PauseRequest {
				skipped: false,
				position_in_milliseconds: 1027,
			}
			.into(),
		)
		.await
		.expect("Failed to get success response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = PlaybackStateChangedBroadcast {
			changed_by_name: bob.name().to_string(),
			changed_by_id: bob.id(),
			skipped: false,
			playback_state: PlaybackStateResponse::Paused {
				position_in_milliseconds: 1027,
			},
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_be_able_to_skip_in_paused_mode() {
		let (alice_message_sender, _message_receiver, mut alice_test_client) = WebsocketTestClient::new();
		let (bob_message_sender, _message_receiver, mut bob_test_client) = WebsocketTestClient::new();

		let room = Room::new(2);
		room.add_client("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");
		let bob = room
			.add_client("Bob".to_string(), bob_message_sender)
			.expect("Did not get client handle!");
		room.insert_medium(SomeMedium::FixedLength(FixedLengthMedium::new(
			"Metropolis".to_string(),
			Duration::minutes(153),
		)));

		let response = handle_request(
			&room,
			&bob,
			PauseRequest {
				skipped: true,
				position_in_milliseconds: 1000,
			}
			.into(),
		)
		.await
		.expect("Failed to get success response");
		assert_eq!(response, SuccessMessage::Success);

		let alice_broadcast = alice_test_client.receive_broadcast_message().await;
		let bob_broadcast = bob_test_client.receive_broadcast_message().await;

		let expected_broadcast = PlaybackStateChangedBroadcast {
			changed_by_name: bob.name().to_string(),
			changed_by_id: bob.id(),
			skipped: true,
			playback_state: PlaybackStateResponse::Paused {
				position_in_milliseconds: 1000,
			},
		};

		assert_eq!(alice_broadcast, expected_broadcast.clone().into());
		assert_eq!(bob_broadcast, expected_broadcast.into());
	}

	#[tokio::test]
	async fn the_client_should_not_be_able_to_pause_something_without_medium() {
		let (alice_message_sender, _message_receiver, _alice_test_client) = WebsocketTestClient::new();

		let room = Room::new(1);
		let alice = room
			.add_client("Alice".to_string(), alice_message_sender)
			.expect("Did not get client handle!");

		let response = handle_request(
			&room,
			&alice,
			PauseRequest {
				skipped: false,
				position_in_milliseconds: 1000,
			}
			.into(),
		)
		.await
		.expect_err("Failed to get error response");

		assert_eq!(
			response,
			ErrorMessage::builder()
				.error(ErrorMessageType::NoMedium)
				.message("Room has no medium.".to_string())
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
				handle_messages(room, client_handle, message_receiver).await
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
		let fake_message_sender = FakeClientConnection::default().into();
		room.add_client("Ferris".to_string(), fake_message_sender)
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
			let message_sender = MessageSender::from(FakeClientConnection::default());
			room.add_client("Fake".to_string(), message_sender).unwrap();
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
		let short_circuit = SomeMedium::FixedLength(FixedLengthMedium::new(video_name.clone(), video_length));
		room.insert_medium(short_circuit);
		room.play_medium(Duration::milliseconds(0));

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
				current_medium: Some(MediumResponse::FixedLength {
					name: video_name,
					length_in_milliseconds: video_length.num_milliseconds() as u64,
					playback_state: PlaybackStateResponse::Playing {
						start_time_in_milliseconds: 0,
					}
				})
			},
			response
		);
	}

	#[tokio::test]
	async fn should_list_other_clients_when_joining_a_room() {
		let room = Room::new(2);
		let fake_message_sender = FakeClientConnection::default();
		let stephanie = room
			.add_client("Stephanie".to_string(), fake_message_sender.into())
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
				current_medium: None,
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
				current_medium: None,
			},
			response
		);
	}

	#[tokio::test]
	async fn should_get_currently_paused_medium_on_hello_response() {
		let room = Room::new(1);
		let video_name = "Short Circuit".to_string();
		let video_length = Duration::minutes(98);
		let short_circuit = SomeMedium::FixedLength(FixedLengthMedium::new(video_name.clone(), video_length));
		room.insert_medium(short_circuit);

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
				current_medium: Some(MediumResponse::FixedLength {
					name: video_name,
					length_in_milliseconds: video_length.num_milliseconds() as u64,
					playback_state: PlaybackStateResponse::Paused {
						position_in_milliseconds: 0
					}
				})
			},
			response
		);
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
		let (client_handle, message_receiver) = register_client(room.clone(), message_sender, message_receiver)
			.await
			.unwrap();

		let response = test_client.receive_success_message(request_id).await;

		let id = if let SuccessMessage::Hello { id, .. } = response {
			id
		} else {
			panic!("Expected Hello-Response, got '{:?}'", response);
		};
		assert_eq!(client_handle.id(), id);

		let joined_response = test_client.receive_broadcast_message().await;
		assert!(matches!(
			joined_response,
			BroadcastMessage::ClientJoined(ClientJoinedBroadcast { id: _, name: _ })
		));
		(client_handle, message_receiver, test_client)
	}
}
