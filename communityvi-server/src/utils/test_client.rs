use crate::connection::receiver::{MessageReceiver, StreamMessageReceiver};
use crate::connection::sender::{MessageSender, SinkMessageSender};
use crate::message::client_request::{ClientRequest, ClientRequestWithId};
use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use crate::message::outgoing::OutgoingMessage;
use crate::message::WebSocketMessage;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::convert::TryFrom;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;

pub struct WebsocketTestClient {
	sender: Pin<Box<dyn Sink<WebSocketMessage, Error = ()> + Unpin + Send>>,
	receiver: Pin<Box<dyn Stream<Item = WebSocketMessage> + Unpin + Send>>,
}

impl WebsocketTestClient {
	pub fn new() -> (MessageSender, MessageReceiver, Self) {
		let (client_sender, server_receiver) = futures::channel::mpsc::unbounded();
		let (server_sender, client_receiver) = futures::channel::mpsc::unbounded();
		let client_sender = client_sender.sink_map_err(|_error| ());

		let sink_message_sender = SinkMessageSender::new(server_sender);
		let message_sender = MessageSender::from(sink_message_sender);
		let stream_message_receiver = StreamMessageReceiver::new(server_receiver, message_sender.clone());

		let message_receiver = MessageReceiver::from(stream_message_receiver);

		let test_client = Self {
			sender: Box::pin(client_sender),
			receiver: Box::pin(client_receiver),
		};

		(message_sender, message_receiver, test_client)
	}

	pub async fn send_raw(&mut self, message: WebSocketMessage) {
		self.sender
			.send(message)
			.await
			.expect("Failed to send message via TestClient.");
	}

	pub async fn receive_raw(&mut self) -> WebSocketMessage {
		self.receiver
			.next()
			.await
			.expect("Failed to receive message via TestClient")
	}

	pub async fn send_request(&mut self, request: impl Into<ClientRequest>) -> u64 {
		let request_id = rand::random();
		self.send_request_with_id(request, request_id).await;
		request_id
	}

	pub async fn send_request_with_id(&mut self, request: impl Into<ClientRequest>, request_id: u64) {
		let client_request = ClientRequestWithId {
			request_id,
			request: request.into(),
		};
		let websocket_message = WebSocketMessage::from(&client_request);
		self.send_raw(websocket_message).await
	}

	pub async fn receive_success_message(&mut self, expected_request_id: u64) -> SuccessMessage {
		let websocket_message = self.receive_raw().await;
		match OutgoingMessage::try_from(&websocket_message).expect("Failed to deserialize OutgoingMessage") {
			OutgoingMessage::Success { request_id, message } => {
				assert_eq!(request_id, expected_request_id);
				message
			}
			message @ _ => panic!("Received message with incorrect type: {:?}", message),
		}
	}

	pub async fn receive_error_message(&mut self, expected_request_id: Option<u64>) -> ErrorMessage {
		let websocket_message = self.receive_raw().await;
		match OutgoingMessage::try_from(&websocket_message).expect("Failed to deserialize OutgoingMessage") {
			OutgoingMessage::Error { request_id, message } => {
				assert_eq!(request_id, expected_request_id);
				message
			}
			message @ _ => panic!("Received message with incorrect type: {:?}", message),
		}
	}

	pub async fn receive_broadcast_message(&mut self) -> BroadcastMessage {
		let websocket_message = self.receive_raw().await;
		match OutgoingMessage::try_from(&websocket_message).expect("Failed to deserialize OutgoingMessage") {
			OutgoingMessage::Broadcast { message } => message,
			message @ _ => panic!("Received message with incorrect type: {:?}", message),
		}
	}
}

impl<Socket> From<WebSocketStream<Socket>> for WebsocketTestClient
where
	Socket: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
	fn from(websocket: WebSocketStream<Socket>) -> Self {
		let (sender, receiver) = websocket.split();
		let sender = sender.sink_map_err(|_error| ());
		let receiver = receiver.map(|result| result.expect("Failed to receive websocket message"));
		Self {
			sender: Box::pin(sender),
			receiver: Box::pin(receiver),
		}
	}
}
