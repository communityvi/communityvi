use crate::connection::sender::MessageSender;
use crate::message::client_request::{ClientRequestWithId, RequestIdOnly};
use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::{MessageError, WebSocketMessage};
use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use tracing::error;

pub struct MessageReceiver {
	stream: Pin<Box<dyn Stream<Item = anyhow::Result<WebSocketMessage>> + Unpin + Send>>,
	sender: MessageSender,
}

impl MessageReceiver {
	pub fn new<WebSocketStream>(websocket_stream: WebSocketStream, sender: MessageSender) -> Self
	where
		WebSocketStream: Stream<Item = anyhow::Result<WebSocketMessage>> + Unpin + Send + 'static,
	{
		Self {
			stream: Box::pin(websocket_stream),
			sender,
		}
	}

	/// Receive a message from the client or Finished if the connection has been closed.
	pub async fn receive(&mut self) -> ReceivedMessage {
		const MAXIMUM_RETRIES: usize = 10;
		use ReceivedMessage::Finished;

		for _ in 0..MAXIMUM_RETRIES {
			use tokio_tungstenite::tungstenite::Message::*;
			let websocket_message = loop {
				match self.stream.next().await {
					Some(Ok(Ping(payload))) => {
						// respond to ping in-place. Previously, tungstenite did this for us, but now we need to
						// to it manually and keep on looping until we receive a non-ping message
						// TODO: Put this responding into a more appropriate place
						if let Err(()) = self.sender.send_pong(payload.into()).await {
							return Finished;
						}
					}
					Some(Ok(websocket_message)) => break websocket_message,
					Some(Err(error)) => {
						error!("Failed to receive websocket message: {error}");
						return Finished;
					}
					None => return Finished,
				}
			};

			let websocket_message = match websocket_message {
				Pong(payload) => {
					return ReceivedMessage::Pong {
						payload: payload.to_vec(),
					};
				}
				Close(_) => {
					self.sender.close().await;
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
						MessageError::DeserializationFailed { error, json } => {
							format!("Failed to deserialize client message with error: {error}, message was: {json}")
						}
						MessageError::WrongMessageType(message) => {
							format!("Client request has incorrect message type. Message was: {message:?}")
						}
					};
					error!("{message}");
					let _ = self
						.sender
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
			.sender
			.send_error_message(
				ErrorMessage::builder()
					.error(ErrorMessageType::InvalidOperation)
					.message("Too many retries".to_string())
					.build(),
				None,
			)
			.await;
		self.sender.close().await;
		Finished
	}
}

#[derive(Debug, PartialEq, Eq)]
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
