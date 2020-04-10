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
	}
}
