use crate::connection::client::ClientConnection;
use crate::message::{ClientRequest, ErrorResponse, MessageError, OrderedMessage, ServerResponse};
use futures::stream::SplitStream;
use futures::StreamExt;
use log::error;
use std::convert::TryFrom;
use warp::ws::WebSocket;

pub struct ServerConnection {
	websocket_stream: SplitStream<WebSocket>,
	client_connection: ClientConnection,
}

impl ServerConnection {
	pub fn new(websocket_stream: SplitStream<WebSocket>, client_connection: ClientConnection) -> Self {
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
