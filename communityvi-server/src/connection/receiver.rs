use crate::connection::sender::MessageSender;
use crate::message::client_request::{ClientRequestWithId, RequestIdOnly};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::{MessageError, WebSocketMessage};
use futures::{Stream, StreamExt};
use log::error;
use std::convert::TryFrom;
use std::pin::Pin;

pub struct MessageReceiver {
	stream: Pin<Box<dyn Stream<Item = WebSocketMessage> + Unpin + Send>>,
	message_sender: MessageSender,
}

impl MessageReceiver {
	pub fn new<RequestStream>(request_stream: RequestStream, message_sender: MessageSender) -> Self
	where
		RequestStream: Stream<Item = WebSocketMessage> + Unpin + Send + 'static,
	{
		Self {
			stream: Box::pin(request_stream),
			message_sender,
		}
	}

	/// Receive a message from the client or Finished if the connection has been closed.
	pub async fn receive(&mut self) -> ReceivedMessage {
		const MAXIMUM_RETRIES: usize = 10;
		use ReceivedMessage::Finished;

		for _ in 0..MAXIMUM_RETRIES {
			let websocket_message = match self.stream.next().await {
				Some(websocket_message) => websocket_message,
				None => return Finished,
			};

			use tokio_tungstenite::tungstenite::Message::*;
			let websocket_message = match websocket_message {
				Pong(payload) => return ReceivedMessage::Pong { payload },
				Close(_) => {
					self.message_sender.close().await;
					return Finished;
				}
				websocket_message => websocket_message,
			};

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
						.message_sender
						.send_error_message(
							ErrorMessage::builder()
								.error(ErrorMessageType::InvalidFormat)
								.message(message)
								.build(),
							request_id,
						)
						.await;
					continue;
				}
			};

			return client_request.into();
		}

		let _ = self
			.message_sender
			.send_error_message(
				ErrorMessage::builder()
					.error(ErrorMessageType::InvalidOperation)
					.message("Too many retries".to_string())
					.build(),
				None,
			)
			.await;
		self.message_sender.close().await;
		Finished
	}
}

#[derive(Debug, PartialEq)]
pub enum ReceivedMessage {
	Request(ClientRequestWithId),
	Pong { payload: Vec<u8> },
	Finished,
}

impl From<ClientRequestWithId> for ReceivedMessage {
	fn from(request: ClientRequestWithId) -> Self {
		ReceivedMessage::Request(request)
	}
}

#[cfg(test)]
pub mod test {
	use std::time::Duration;

	#[tokio::test]
	async fn validate_that_tokio_test_does_not_wait_for_completion() {
		tokio::spawn(tokio::time::sleep(Duration::from_secs(10)));
	}
}
