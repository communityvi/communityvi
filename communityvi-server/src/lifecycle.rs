use crate::client::{Client, ClientId};
use crate::connection::client::ClientConnection;
use crate::connection::server::ServerConnection;
use crate::connection::split_websocket;
use crate::message::{ClientRequest, ErrorResponse, OrderedMessage, ServerResponse};
use crate::room::Room;
use log::{debug, error, info, warn};
use warp::filters::ws::WebSocket;

pub async fn run_client(room: &Room, websocket: WebSocket) {
	let (client_connection, server_connection) = split_websocket(websocket);
	if let Some((client_id, server_connection)) = register_client(&room, client_connection, server_connection).await {
		handle_messages(server_connection, client_id, &room).await;
		room.remove_client(client_id).await;
	}
}

async fn register_client(
	room: &Room,
	client_connection: ClientConnection,
	mut server_connection: ServerConnection,
) -> Option<(ClientId, ServerConnection)> {
	let request = match server_connection.receive().await {
		None => {
			error!("Client registration failed. Socket closed prematurely.");
			return None;
		}
		Some(request) => request,
	};

	let (number, name) = if let OrderedMessage {
		number,
		message: ClientRequest::Register { name },
	} = request
	{
		(number, name)
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

	if number != 0 {
		let message = format!(
			"Client registration failed. Invalid message number: {}, should be 0.",
			number
		);
		error!("{}", message);
		let _ = client_connection
			.send(ServerResponse::Error {
				error: ErrorResponse::InvalidOperation,
				message,
			})
			.await;
		return None;
	}

	let client_handle = room.add_client(name, client_connection);
	let hello_response = ServerResponse::Hello { id: client_handle.id() };
	if room.singlecast(&client_handle, hello_response).await.is_ok() {
		let name = client_handle.name().to_string();
		let id = client_handle.id();

		// Drop the client_handle so that the lock on the concurrent hashmap is released for the broadcast
		std::mem::drop(client_handle);

		info!("Registered client: {} {}", id, name);

		room.broadcast(ServerResponse::Joined { id, name }).await;

		Some((id, server_connection))
	} else {
		None
	}
}

async fn handle_messages(mut server_connection: ServerConnection, client_id: ClientId, room: &Room) {
	loop {
		let OrderedMessage { number, message } = match server_connection.receive().await {
			Some(message) => message,
			None => return,
		};
		debug!(
			"Received {:?} message {} from {}",
			std::mem::discriminant(&message),
			number,
			client_id
		);

		let client = match room.get_client_by_id(client_id) {
			Some(client_handle) => client_handle,
			None => {
				warn!("Couldn't find Client: {}", client_id);
				return;
			}
		};
		handle_message(room, &client, message).await;
	}
}

async fn handle_message(room: &Room, client: &Client, request: ClientRequest) {
	match request {
		ClientRequest::Ping => {
			let _ = room.singlecast(&client, ServerResponse::Pong).await;
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
			let _ = room
				.singlecast(
					&client,
					ServerResponse::Error {
						error: ErrorResponse::InvalidOperation,
						message: "Already registered".to_string(),
					},
				)
				.await;
		}
		ClientRequest::GetReferenceTime => {
			let reference_time = room.current_reference_time();
			let message = ServerResponse::ReferenceTime {
				milliseconds: reference_time.as_millis() as u64,
			};
			let _ = client.send(message).await;
		}
	}
}

#[cfg(test)]
mod test {
	use crate::client::ClientId;
	use crate::connection::client::ClientConnection;
	use crate::connection::server::ServerConnection;
	use crate::connection::test::{create_typed_test_connections, TypedClientSinkStream};
	use crate::lifecycle::{handle_message, handle_messages, register_client};
	use crate::message::{ClientRequest, ErrorResponse, OrderedMessage, ServerResponse};
	use crate::room::Room;
	use futures::{SinkExt, StreamExt};
	use std::time::Duration;
	use tokio::time::delay_for;

	#[tokio::test]
	async fn the_client_should_get_access_to_the_server_reference_time() {
		const TEST_DELAY: Duration = Duration::from_millis(2);

		let (client_connection, _server_connection, mut client_sink_stream) = create_typed_test_connections();
		let room = Room::default();
		let client_handle = room.add_client("Alice".to_string(), client_connection);

		delay_for(TEST_DELAY).await; // ensure that some time has passed
		handle_message(&room, &client_handle, ClientRequest::GetReferenceTime).await;

		match client_sink_stream
			.next()
			.await
			.unwrap()
			.expect("Invalid ordered message")
		{
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
	async fn should_enforce_zero_message_numbers_during_registration() {
		let (client_connection, server_connection, mut client_sink_stream) = create_typed_test_connections();
		let room = Room::default();

		let register_message = OrderedMessage {
			number: 1,
			message: ClientRequest::Register {
				name: "Ferris".to_string(),
			},
		};
		client_sink_stream.send(register_message).await.unwrap();

		let option = register_client(&room, client_connection, server_connection).await;
		assert!(option.is_none());

		let response = client_sink_stream.next().await.unwrap().unwrap();

		let expected_response = OrderedMessage {
			number: 0,
			message: ServerResponse::Error {
				error: ErrorResponse::InvalidOperation,
				message: "Client registration failed. Invalid message number: 1, should be 0.".into(),
			},
		};
		assert_eq!(expected_response, response);
	}

	#[tokio::test]
	async fn should_not_allow_registering_client_twice() {
		let (client_connection, server_connection, client_sink_stream) = create_typed_test_connections();
		let room = Room::default();

		let (client_id, server_connection, mut client_sink_stream) = register_test_client(
			"Anorak",
			&room,
			client_connection,
			server_connection,
			client_sink_stream,
		)
		.await;

		// run server message handler
		tokio::spawn(async move { handle_messages(server_connection, client_id, &room).await });

		let register_message = OrderedMessage {
			number: 1,
			message: ClientRequest::Register {
				name: "Parcival".to_string(),
			},
		};

		client_sink_stream
			.send(register_message)
			.await
			.expect("Failed to send second register method.");
		match client_sink_stream
			.next()
			.await
			.unwrap()
			.expect("No response to double register.")
		{
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

	async fn register_test_client(
		name: &'static str,
		room: &Room,
		client_connection: ClientConnection,
		server_connection: ServerConnection,
		mut client_sink_stream: TypedClientSinkStream,
	) -> (ClientId, ServerConnection, TypedClientSinkStream) {
		let register_request = OrderedMessage {
			number: 0,
			message: ClientRequest::Register { name: name.into() },
		};

		client_sink_stream
			.send(register_request)
			.await
			.expect("Failed to send register message.");

		// run server code required for client registration
		let (_client_id, server_connection) = register_client(room, client_connection, server_connection)
			.await
			.unwrap();

		let response = client_sink_stream
			.next()
			.await
			.unwrap()
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

		let joined_response = client_sink_stream
			.next()
			.await
			.unwrap()
			.expect("Failed to get joined response.");
		assert!(
			matches!(joined_response, OrderedMessage {number: _, message: ServerResponse::Joined {id: _, name: _}})
		);
		(id, server_connection, client_sink_stream)
	}
}
