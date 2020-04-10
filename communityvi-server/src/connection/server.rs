use crate::connection::client::ClientConnection;
use crate::message::{ClientRequest, ErrorResponse, MessageError, OrderedMessage, ServerResponse, WebSocketMessage};
use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, Stream, StreamExt};
use log::error;
use std::convert::TryFrom;
use warp::ws::WebSocket;

pub struct ServerConnection<
	RequestStream = SplitStream<WebSocket>,
	ResponseSink = SplitSink<WebSocket, WebSocketMessage>,
> {
	request_stream: RequestStream,
	client_connection: ClientConnection<ResponseSink>,
}

impl<RequestStream, ResponseSink> ServerConnection<RequestStream, ResponseSink>
where
	RequestStream: Stream<Item = Result<WebSocketMessage, warp::Error>> + Unpin,
	ResponseSink: Sink<WebSocketMessage> + Unpin,
{
	pub fn new(request_stream: RequestStream, client_connection: ClientConnection<ResponseSink>) -> Self {
		Self {
			request_stream,
			client_connection,
		}
	}

	/// Receive a message from the client or None if the connection has been closed.
	pub async fn receive(&mut self) -> Option<OrderedMessage<ClientRequest>> {
		const MAXIMUM_RETRIES: usize = 10;

		for _ in 0..MAXIMUM_RETRIES {
			let websocket_message = match self.request_stream.next().await {
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

			let ordered_message = match OrderedMessage::try_from(&websocket_message) {
				Ok(ordered_message) => ordered_message,
				Err(message_error) => {
					let message = match message_error {
						MessageError::DeserializationFailed { error, json } => format!(
							"Failed to deserialize client message with error: {}, message was: {}",
							error, json
						),
						MessageError::WrongMessageType(message) => {
							format!("Client request has incorrect message type. Message was: {:?}", message)
						}
					};
					error!("{}", message);
					let _ = self
						.client_connection
						.send(ServerResponse::Error {
							error: ErrorResponse::InvalidFormat,
							message,
						})
						.await;
					continue;
				}
			};
			return Some(ordered_message);
		}

		let _ = self
			.client_connection
			.send(ServerResponse::Error {
				error: ErrorResponse::InvalidOperation,
				message: "Too many retries".to_string(),
			})
			.await;
		let _ = self.client_connection.close().await;
		None
	}
}
