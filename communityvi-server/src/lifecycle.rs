use crate::client::{Client, ClientId};
use crate::connection::register_client;
use crate::connection::server::ServerConnection;
use crate::message::{ClientRequest, ErrorResponse, OrderedMessage, ServerResponse};
use crate::room::Room;
use log::{debug, error, info, warn};
use warp::filters::ws::WebSocket;

pub async fn run_client(room: &Room, websocket: WebSocket) {
	if let Some((client_id, server_connection)) = register_client(&room, websocket).await {
		handle_messages(server_connection, client_id, &room).await;
		room.remove_client(client_id).await;
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
					},
				)
				.await;
		}
	}
}
