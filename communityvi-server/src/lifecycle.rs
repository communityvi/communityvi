use crate::connection::client::ClientConnection;
use crate::connection::server::ServerConnection;
use crate::message::{ClientRequest, ErrorResponse, ServerResponse};
use crate::room::client::Client;
use crate::room::error::RoomError;
use crate::room::Room;
use log::{debug, error, info};

pub async fn run_client(room: Room, client_connection: ClientConnection, server_connection: ServerConnection) {
	if let Some((client, server_connection)) = register_client(room.clone(), client_connection, server_connection).await
	{
		let client_id = client.id();
		handle_messages(&room, client, server_connection).await;
		room.remove_client(client_id);
	}
}

async fn register_client(
	room: Room,
	client_connection: ClientConnection,
	mut server_connection: ServerConnection,
) -> Option<(Client, ServerConnection)> {
	let request = match server_connection.receive().await {
		None => {
			error!("Client registration failed. Socket closed prematurely.");
			return None;
		}
		Some(request) => request,
	};

	let name = if let ClientRequest::Register { name } = request {
		name
	} else {
		error!("Client registration failed. Invalid request: {:?}", request);

		let _ = client_connection
			.send(ServerResponse::Error {
				error: ErrorResponse::InvalidOperation,
				message: "Invalid request".to_string(),
			})
			.await;
		return None;
	};

	use RoomError::*;
	let client = match room.add_client(name, client_connection.clone()) {
		Ok(client) => client,
		Err(error @ EmptyClientName) | Err(error @ ClientNameAlreadyInUse) | Err(error @ ClientNameTooLong) => {
			error!("Client registration failed. Tried to register with invalid name.");

			let _ = client_connection
				.send(ServerResponse::Error {
					error: ErrorResponse::InvalidFormat,
					message: error.to_string(),
				})
				.await;

			return None;
		}
	};

	let hello_response = ServerResponse::Hello { id: client.id() };
	if client.send(hello_response).await {
		let id = client.id();
		let name = client.name().to_string();

		info!("Registered client: {} {}", id, name);

		room.broadcast(ServerResponse::Joined { id, name }).await;
		Some((client, server_connection))
	} else {
		None
	}
}

async fn handle_messages(room: &Room, client: Client, mut server_connection: ServerConnection) {
	loop {
		let message = match server_connection.receive().await {
			Some(message) => message,
			None => break, // connection has been closed
		};
		debug!(
			"Received {:?} message from {}",
			std::mem::discriminant(&message),
			client.id(),
		);

		handle_message(room, &client, message).await;
	}

	let id = client.id();
	let name = client.name().to_string();
	info!("Client '{}' with id {} has left.", name, id);
	room.broadcast(ServerResponse::Left { id, name }).await;
}

async fn handle_message(room: &Room, client: &Client, request: ClientRequest) {
	match request {
		ClientRequest::Ping => {
			client.send(ServerResponse::Pong).await;
		}
		ClientRequest::Chat { message } => {
			room.broadcast(ServerResponse::Chat {
				sender_id: client.id(),
				sender_name: client.name().to_string(),
				message,
			})
			.await
		}
		ClientRequest::Pong => info!("Received Pong from client: {}", client.id()),
		ClientRequest::Register { .. } => {
			error!(
				"Client: {} tried to register even though it is already registered.",
				client.id()
			);
			client
				.send(ServerResponse::Error {
					error: ErrorResponse::InvalidOperation,
					message: "Already registered".to_string(),
				})
				.await;
		}
		ClientRequest::GetReferenceTime => {
			let reference_time = room.current_reference_time();
			let message = ServerResponse::ReferenceTime {
				milliseconds: reference_time.as_millis() as u64,
			};
			client.send(message).await;
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::connection::test::{create_typed_test_connections, TypedTestClient};
	use crate::lifecycle::{handle_message, handle_messages, register_client};
	use crate::message::OrderedMessage;
	use crate::utils::fake_connection::FakeClientConnection;
	use std::time::Duration;
	use tokio::time::delay_for;

	#[tokio::test]
	async fn the_client_should_get_access_to_the_server_reference_time() {
		const TEST_DELAY: Duration = Duration::from_millis(2);

		let (client_connection, _server_connection, mut test_client) = create_typed_test_connections();
		let room = Room::default();
		let client_handle = room
			.add_client("Alice".to_string(), client_connection)
			.expect("Did not get client handle!");

		delay_for(TEST_DELAY).await; // ensure that some time has passed
		handle_message(&room, &client_handle, ClientRequest::GetReferenceTime).await;

		match test_client.receive().await.expect("Invalid ordered message") {
			OrderedMessage {
				number: _,
				message: ServerResponse::ReferenceTime { milliseconds },
			} => {
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
	async fn should_not_allow_registering_client_twice() {
		let (client_connection, server_connection, test_client) = create_typed_test_connections();
		let room = Room::default();

		let (client_handle, server_connection, mut test_client) = register_test_client(
			"Anorak",
			room.clone(),
			client_connection,
			server_connection,
			test_client,
		)
		.await;

		// run server message handler
		tokio::spawn({
			async move {
				let room = &room;
				handle_messages(room, client_handle, server_connection).await
			}
		});

		let register_message = OrderedMessage {
			number: 1,
			message: ClientRequest::Register {
				name: "Parcival".to_string(),
			},
		};

		test_client.send(register_message).await;
		match test_client.receive().await.expect("No response to double register.") {
			OrderedMessage {
				number,
				message: ServerResponse::Error { error, message },
			} => {
				assert_eq!(2, number);
				assert_eq!(ErrorResponse::InvalidOperation, error);
				assert!(message.contains("registered"));
			}
			_ => panic!("Incorrect message received."),
		}
	}

	#[tokio::test]
	async fn should_not_register_clients_with_blank_name() {
		let (client_connection, server_connection, mut test_client) = create_typed_test_connections();
		let room = Room::default();
		let register_request = OrderedMessage {
			number: 0,
			message: ClientRequest::Register { name: "	 ".to_string() },
		};

		test_client.send(register_request).await;
		register_client(room, client_connection, server_connection).await;
		let response = test_client.receive().await.expect("Did not receive response!");

		assert_eq!(
			OrderedMessage {
				number: 0,
				message: ServerResponse::Error {
					error: ErrorResponse::InvalidFormat,
					message: "Name was empty or whitespace-only.".to_string()
				}
			},
			response
		);
	}

	#[tokio::test]
	async fn should_not_register_clients_with_already_registered_name() {
		let room = Room::default();

		// "Ferris" is already a registered client
		let fake_client_connection = FakeClientConnection::default().into();
		room.add_client("Ferris".to_string(), fake_client_connection)
			.expect("Could not register 'Ferris'!");

		// And I register another client with the same name
		let (client_connection, server_connection, mut test_client) = create_typed_test_connections();
		let register_request = OrderedMessage {
			number: 0,
			message: ClientRequest::Register {
				name: "Ferris".to_string(),
			},
		};

		test_client.send(register_request).await;
		register_client(room, client_connection, server_connection).await;
		let response = test_client.receive().await.expect("Did not receive response!");

		// Then I expect an error
		assert_eq!(
			OrderedMessage {
				number: 0,
				message: ServerResponse::Error {
					error: ErrorResponse::InvalidFormat,
					message: "Client name is already in use.".to_string()
				}
			},
			response
		);
	}

	async fn register_test_client(
		name: &'static str,
		room: Room,
		client_connection: ClientConnection,
		server_connection: ServerConnection,
		mut test_client: TypedTestClient,
	) -> (Client, ServerConnection, TypedTestClient) {
		let register_request = OrderedMessage {
			number: 0,
			message: ClientRequest::Register { name: name.into() },
		};

		test_client.send(register_request).await;

		// run server code required for client registration
		let (client_handle, server_connection) = register_client(room.clone(), client_connection, server_connection)
			.await
			.unwrap();

		let response = test_client
			.receive()
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
		assert_eq!(client_handle.id(), id);

		let joined_response = test_client.receive().await.expect("Failed to get joined response.");
		assert!(
			matches!(joined_response, OrderedMessage {number: _, message: ServerResponse::Joined {id: _, name: _}})
		);
		(client_handle, server_connection, test_client)
	}
}
