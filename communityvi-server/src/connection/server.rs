use crate::connection::client::ClientConnection;
use crate::message::{ClientRequest, ErrorResponse, MessageError, OrderedMessage, ServerResponse, WebSocketMessage};
use crate::utils::infallible_stream::InfallibleStream;
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
	highest_message_number: Option<u64>,
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

			match self.highest_message_number {
				Some(highest_message_number) if highest_message_number >= ordered_message.number => {
					let message = format!(
						"Received message number '{}', expected larger than '{}'!",
						ordered_message.number, highest_message_number
					);
					error!("{}", message);
					let _ = self
						.client_connection
						.send(ServerResponse::Error {
							error: ErrorResponse::InvalidMessageNumber,
							message,
						})
						.await;

					continue;
				}
				None if ordered_message.number != 0 => {
					let message = format!(
						"Received message number '{}', conversation must start with '0'!",
						ordered_message.number
					);
					error!("{}", message);
					let _ = self
						.client_connection
						.send(ServerResponse::Error {
							error: ErrorResponse::InvalidMessageNumber,
							message,
						})
						.await;

					continue;
				}
				_ => self.highest_message_number = Some(ordered_message.number),
			}

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
			highest_message_number: None,
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

#[cfg(test)]
pub mod test {
	use super::test_helper::*;
	use super::*;
	use futures::SinkExt;
	use std::time::Duration;

	#[tokio::test]
	async fn validate_that_tokio_test_does_not_wait_for_completion() {
		tokio::spawn(tokio::time::delay_for(Duration::from_secs(10)));
	}

	#[tokio::test]
	async fn should_reject_nonzero_message_numbers_at_start_of_conversation() {
		async_test_case_receiving(0, |mut client_sink_stream| async move {
			let nonzero_message = some_ordered_message(42);

			client_sink_stream.send(nonzero_message).await.unwrap();
			let response = client_sink_stream.next().await.unwrap().unwrap();

			assert_eq!(
				OrderedMessage {
					number: 0,
					message: ServerResponse::Error {
						error: ErrorResponse::InvalidMessageNumber,
						message: "Received message number '42', conversation must start with '0'!".to_string()
					}
				},
				response
			);
		})
		.await;
	}

	#[tokio::test]
	async fn should_reject_messages_with_message_numbers_lower_than_the_previous() {
		async_test_case_receiving(2, |mut client_sink_stream| async move {
			let first_message = some_ordered_message(0);
			let second_message = some_ordered_message(1);
			let message_with_lower_number = some_ordered_message(0);

			client_sink_stream.send(first_message).await.unwrap();
			client_sink_stream.send(second_message).await.unwrap();
			client_sink_stream.send(message_with_lower_number).await.unwrap();
			let response = client_sink_stream.next().await.unwrap().unwrap();

			assert_eq!(
				OrderedMessage {
					number: 0,
					message: ServerResponse::Error {
						error: ErrorResponse::InvalidMessageNumber,
						message: "Received message number '0', expected larger than '1'!".to_string()
					}
				},
				response
			);
		})
		.await;
	}

	#[tokio::test]
	async fn should_reject_messages_with_message_numbers_identical_to_the_previous() {
		async_test_case_receiving(1, |mut client_sink_stream| async move {
			let first_message = some_ordered_message(0);
			let first_message_sent_again = first_message.clone();

			client_sink_stream.send(first_message).await.unwrap();
			client_sink_stream.send(first_message_sent_again).await.unwrap();
			let response = client_sink_stream.next().await.unwrap().unwrap();

			assert_eq!(
				OrderedMessage {
					number: 0,
					message: ServerResponse::Error {
						error: ErrorResponse::InvalidMessageNumber,
						message: "Received message number '0', expected larger than '0'!".to_string()
					}
				},
				response
			);
		})
		.await;
	}

	#[tokio::test]
	async fn should_accept_conversations_beginning_with_message_number_zero() {
		async_test_case_receiving(1, |mut client_sink_stream| async move {
			let first_message = some_ordered_message(0);

			client_sink_stream.send(first_message).await.unwrap();
		})
		.await;
	}

	#[tokio::test]
	async fn should_accept_strictly_increasing_message_numbers() {
		async_test_case_receiving(4, |mut client_sink_stream| async move {
			let first_message = some_ordered_message(0);
			let second_message = some_ordered_message(1);
			let tenth_message = some_ordered_message(9);
			let onethousandthreehundredthirtyeigth_message = some_ordered_message(1337);

			client_sink_stream.send(first_message).await.unwrap();
			client_sink_stream.send(second_message).await.unwrap();
			client_sink_stream.send(tenth_message).await.unwrap();
			client_sink_stream
				.send(onethousandthreehundredthirtyeigth_message)
				.await
				.unwrap();
		})
		.await;
	}
}

#[cfg(test)]
mod test_helper {
	use crate::connection::test::{create_typed_test_connections, TypedClientSinkStream};
	use crate::message::{ClientRequest, OrderedMessage};
	use std::future::Future;

	pub async fn async_test_case_receiving<TestCaseClosure, TestFuture>(
		expected_valid_message_count: usize,
		test_case: TestCaseClosure,
	) where
		TestCaseClosure: FnOnce(TypedClientSinkStream) -> TestFuture,
		TestFuture: Future<Output = ()>,
	{
		let (_, mut server_connection, client_sink_stream) = create_typed_test_connections();
		let join_handle = tokio::spawn(async move {
			for current_count in 1..=expected_valid_message_count {
				if server_connection.receive().await.is_none() {
					panic!(
						"Expected {} valid messages, received only {}.",
						expected_valid_message_count,
						(current_count - 1)
					);
				}
			}
			assert!(
				server_connection.receive().await.is_none(),
				"Expected only {} messages, but received at least one more.",
				expected_valid_message_count
			);
		});
		test_case(client_sink_stream).await;
		join_handle.await.expect("Failed to finish server.");
	}

	pub fn some_ordered_message(number: u64) -> OrderedMessage<ClientRequest> {
		OrderedMessage {
			number,
			message: ClientRequest::Ping,
		}
	}
}
