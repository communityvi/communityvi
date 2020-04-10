use crate::client::ClientId;
use crate::connection::client::ClientConnection;
use crate::connection::server::ServerConnection;
use crate::message::{ClientRequest, ErrorResponse, OrderedMessage, ServerResponse};
use crate::room::Room;
use futures::StreamExt;
use log::{error, info};
use warp::filters::ws::WebSocket;

pub mod client;
pub mod server;

pub async fn register_client(room: &Room, websocket: WebSocket) -> Option<(ClientId, ServerConnection)> {
	let (client_connection, mut server_connection) = split_websocket(websocket);
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
			})
			.await;
		return None;
	};

	if number != 0 {
		error!(
			"Client registration failed. Invalid message number: {}, should be 0.",
			number
		);
		let _ = client_connection
			.send(ServerResponse::Error {
				error: ErrorResponse::InvalidOperation,
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

fn split_websocket(websocket: WebSocket) -> (ClientConnection, ServerConnection) {
	let (websocket_sink, websocket_stream) = websocket.split();
	let client_connection = ClientConnection::new(websocket_sink);
	let server_connection = ServerConnection::new(websocket_stream, client_connection.clone());
	(client_connection, server_connection)
}
