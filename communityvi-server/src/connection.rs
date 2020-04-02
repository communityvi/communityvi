use crate::client::ClientId;
use crate::message::{ClientRequest, ErrorResponse, MessageError, OrderedMessage, ServerResponse, WebSocketMessage};
use crate::room::Room;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{error, info};
use std::convert::TryFrom;
use std::ops::Range;
use std::sync::Arc;
use warp::filters::ws::WebSocket;

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

pub struct ServerConnection {
	websocket_stream: SplitStream<WebSocket>,
	client_connection: ClientConnection,
}

impl ServerConnection {
	fn new(websocket_stream: SplitStream<WebSocket>, client_connection: ClientConnection) -> Self {
		Self {
			websocket_stream,
			client_connection,
		}
	}

	/// Receive a message from the client or None if the connection has been closed.
	pub async fn receive(&mut self) -> Option<OrderedMessage<ClientRequest>> {
		let websocket_message = match self.websocket_stream.next().await {
			Some(Ok(websocket_message)) => websocket_message,
			Some(Err(error)) => {
				error!("Error streaming websocket message: {}, result.", error);
				return None;
			}
			None => return None,
		};

		if websocket_message.is_close() {
			self.client_connection.close().await;
			return None;
		}

		match OrderedMessage::try_from(&websocket_message) {
			Ok(ordered_message) => Some(ordered_message),
			Err(message_error) => {
				match message_error {
					MessageError::DeserializationFailed { error, json } => {
						error!(
							"Failed to deserialize client message with error: {}, message was: {}",
							error, json
						);
					}
					MessageError::WrongMessageType(message) => {
						error!("Client request has incorrect message type. Message was: {:?}", message);
					}
				}
				let _ = self
					.client_connection
					.send(ServerResponse::Error {
						error: ErrorResponse::InvalidFormat,
					})
					.await;
				None
			}
		}
	}
}

#[derive(Clone, Debug)]
pub struct ClientConnection {
	inner: Arc<tokio::sync::Mutex<ClientConnectionInner>>,
}

#[derive(Debug)]
struct ClientConnectionInner {
	websocket_sink: SplitSink<WebSocket, WebSocketMessage>,
	message_number_sequence: Range<u64>,
}

impl ClientConnection {
	fn new(websocket_sink: SplitSink<WebSocket, WebSocketMessage>) -> Self {
		let inner = ClientConnectionInner {
			websocket_sink,
			message_number_sequence: (0..std::u64::MAX),
		};
		Self {
			inner: Arc::new(inner.into()),
		}
	}

	pub async fn send(&self, message: ServerResponse) -> Result<(), ()> {
		let mut inner = self.inner.lock().await;

		let ordered_message = OrderedMessage {
			number: inner.message_number_sequence.next().expect("Out of message numbers"),
			message,
		};
		let websocket_message = WebSocketMessage::from(&ordered_message);

		inner.websocket_sink.send(websocket_message).await.map_err(|_| ())
	}

	async fn close(&self) {
		let mut inner = self.inner.lock().await;
		let _ = inner.websocket_sink.send(WebSocketMessage::close()).await;
	}
}
