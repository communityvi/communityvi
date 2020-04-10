use crate::connection::client::ClientConnection;
use crate::infallible_stream::InfallibleStream;
use crate::message::{ClientRequest, ErrorResponse, MessageError, OrderedMessage, ServerResponse, WebSocketMessage};
use async_trait::async_trait;
use futures::stream::SplitStream;
use futures::{Stream, StreamExt};
use log::error;
use std::convert::TryFrom;
use std::pin::Pin;
use warp::ws::WebSocket;

pub type ServerConnection = Pin<Box<dyn ServerConnectionTrait + Unpin + Send>>;
pub type WebSocketServerConnection = StreamServerConnection<InfallibleStream<SplitStream<WebSocket>>>;

#[async_trait]
pub trait ServerConnectionTrait {
	/// Receive a message from the client or None if the connection has been closed.
	async fn receive(&mut self) -> Option<OrderedMessage<ClientRequest>>;
}

pub struct StreamServerConnection<RequestStream = InfallibleStream<SplitStream<WebSocket>>> {
	request_stream: RequestStream,
	client_connection: ClientConnection,
}

#[async_trait]
impl<RequestStream> ServerConnectionTrait for StreamServerConnection<RequestStream>
where
	RequestStream: Stream<Item = WebSocketMessage> + Unpin + Send,
{
	async fn receive(&mut self) -> Option<OrderedMessage<ClientRequest>> {
		const MAXIMUM_RETRIES: usize = 10;

		for _ in 0..MAXIMUM_RETRIES {
			let websocket_message = match self.request_stream.next().await {
				Some(websocket_message) => websocket_message,
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

impl<RequestStream> StreamServerConnection<RequestStream>
where
	RequestStream: Stream<Item = WebSocketMessage>,
{
	pub fn new(request_stream: RequestStream, client_connection: ClientConnection) -> Self {
		Self {
			request_stream,
			client_connection,
		}
	}
}

impl<RequestStream> From<StreamServerConnection<RequestStream>> for ServerConnection
where
	RequestStream: Stream<Item = WebSocketMessage> + Unpin + Send + 'static,
{
	fn from(stream_server_connection: StreamServerConnection<RequestStream>) -> Self {
		Box::pin(stream_server_connection)
	}
}
