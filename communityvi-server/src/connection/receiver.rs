use crate::connection::sender::MessageSender;
use crate::message::client_request::{ClientRequestWithId, RequestIdOnly};
use crate::message::server_response::{ErrorResponse, ServerResponseWithId};
use crate::message::{server_response::ErrorResponseType, MessageError, WebSocketMessage};
use crate::server::WebSocket;
use crate::utils::infallible_stream::InfallibleStream;
use async_trait::async_trait;
use futures::stream::SplitStream;
use futures::{Stream, StreamExt};
use log::error;
use std::convert::TryFrom;
use std::pin::Pin;

pub type MessageReceiver = Pin<Box<dyn MessageReceiverTrait + Unpin + Send>>;
pub type WebSocketMessageReceiver = StreamMessageReceiver<InfallibleStream<SplitStream<WebSocket>>>;

#[async_trait]
pub trait MessageReceiverTrait {
	/// Receive a message from the client or None if the connection has been closed.
	async fn receive(&mut self) -> Option<ClientRequestWithId>;
}

pub struct StreamMessageReceiver<RequestStream = InfallibleStream<SplitStream<WebSocket>>> {
	request_stream: RequestStream,
	client_connection: MessageSender,
}

#[async_trait]
impl<RequestStream> MessageReceiverTrait for StreamMessageReceiver<RequestStream>
where
	RequestStream: Stream<Item = WebSocketMessage> + Unpin + Send,
{
	async fn receive(&mut self) -> Option<ClientRequestWithId> {
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

			let client_request = match ClientRequestWithId::try_from(&websocket_message) {
				Ok(client_request) => client_request,
				Err(message_error) => {
					let request_id = RequestIdOnly::try_from(&websocket_message)
						.map(|request| request.request_id)
						.ok();
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
						.send_response(ServerResponseWithId {
							request_id,
							response: ErrorResponse {
								error: ErrorResponseType::InvalidFormat,
								message,
							}
							.into(),
						})
						.await;
					continue;
				}
			};

			return Some(client_request);
		}

		let _ = self
			.client_connection
			.send_response(ServerResponseWithId {
				request_id: None,
				response: ErrorResponse {
					error: ErrorResponseType::InvalidOperation,
					message: "Too many retries".to_string(),
				}
				.into(),
			})
			.await;
		let _ = self.client_connection.close().await;
		None
	}
}

impl<RequestStream> StreamMessageReceiver<RequestStream>
where
	RequestStream: Stream<Item = WebSocketMessage>,
{
	pub fn new(request_stream: RequestStream, client_connection: MessageSender) -> Self {
		Self {
			request_stream,
			client_connection,
		}
	}
}

impl<RequestStream> From<StreamMessageReceiver<RequestStream>> for MessageReceiver
where
	RequestStream: Stream<Item = WebSocketMessage> + Unpin + Send + 'static,
{
	fn from(stream_server_connection: StreamMessageReceiver<RequestStream>) -> Self {
		Box::pin(stream_server_connection)
	}
}

#[cfg(test)]
pub mod test {
	use std::time::Duration;

	#[tokio::test]
	async fn validate_that_tokio_test_does_not_wait_for_completion() {
		tokio::spawn(tokio::time::delay_for(Duration::from_secs(10)));
	}
}
