use crate::connection::receiver::MessageReceiver;
use crate::connection::sender::MessageSender;
use crate::lifecycle::send_broadcasts;
use crate::message::client_request::{ClientRequest, ClientRequestWithId};
use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use crate::message::outgoing::OutgoingMessage;
use crate::message::WebSocketMessage;
use crate::room::client::Client;
use crate::room::Room;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::collections::{BTreeMap, VecDeque};
use std::convert::TryFrom;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;

pub struct WebsocketTestClient {
	sender: Pin<Box<dyn Sink<WebSocketMessage, Error = ()> + Unpin + Send>>,
	receiver: Pin<Box<dyn Stream<Item = WebSocketMessage> + Unpin + Send>>,
	success_messages: BTreeMap<u64, SuccessMessage>,
	error_messages: BTreeMap<Option<u64>, ErrorMessage>,
	broadcast_messages: VecDeque<BroadcastMessage>,
}

impl WebsocketTestClient {
	pub fn new() -> (MessageSender, MessageReceiver, Self) {
		let (client_sender, server_receiver) = futures::channel::mpsc::unbounded();
		let (server_sender, client_receiver) = futures::channel::mpsc::unbounded();
		let client_sender = client_sender.sink_map_err(|_error| ());

		let message_sender = MessageSender::from(server_sender.sink_map_err(Into::into));
		let message_receiver = MessageReceiver::new(server_receiver.map(Result::Ok), message_sender.clone());

		let test_client = Self {
			sender: Box::pin(client_sender),
			receiver: Box::pin(client_receiver),
			success_messages: Default::default(),
			error_messages: Default::default(),
			broadcast_messages: Default::default(),
		};

		(message_sender, message_receiver, test_client)
	}

	// async because it uses tokio::spawn. This make it clear that this should not be run outside of a runtime.
	pub async fn in_room(name: &'static str, room: &Room) -> (Client, Self) {
		let (sender, _, test_client) = Self::new();
		let (client, _) = room
			.add_client_and_return_existing(name.to_string(), sender)
			.expect("Failed to add client to room");
		tokio::spawn(send_broadcasts(client.clone()));
		(client, test_client)
	}

	pub async fn send_raw(&mut self, message: WebSocketMessage) {
		self.sender
			.send(message)
			.await
			.expect("Failed to send message via TestClient.");
	}

	pub async fn receive_raw(&mut self) -> WebSocketMessage {
		timeout(Duration::from_secs(1), self.receiver.next())
			.await
			.expect("Timed out waiting for raw message.")
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
		loop {
			self.receive_outgoing_message().await;
			if let Some(success) = self.success_messages.remove(&expected_request_id) {
				return success;
			}
		}
	}

	pub async fn receive_error_message(&mut self, expected_request_id: Option<u64>) -> ErrorMessage {
		loop {
			self.receive_outgoing_message().await;
			if let Some(error) = self.error_messages.remove(&expected_request_id) {
				return error;
			}
		}
	}

	pub async fn receive_broadcast_message(&mut self) -> BroadcastMessage {
		loop {
			self.receive_outgoing_message().await;
			if let Some(message) = self.broadcast_messages.pop_front() {
				return message;
			}
		}
	}

	pub async fn receive_ping(&mut self) -> Vec<u8> {
		if let WebSocketMessage::Ping(payload) = self.receive_raw().await {
			payload
		} else {
			panic!("Invalid raw message received.");
		}
	}

	async fn receive_outgoing_message(&mut self) {
		let websocket_message = timeout(Duration::from_secs(1), self.receive_raw())
			.await
			.expect("Timeout while waiting for message.");
		use OutgoingMessage::*;
		match OutgoingMessage::try_from(&websocket_message).expect("Failed to deserialize OutgoingMessage") {
			Success { request_id, message } => {
				self.success_messages.insert(request_id, message);
			}
			Error { request_id, message } => assert!(
				self.error_messages.insert(request_id, message).is_none(),
				"Tried to queue more than one error message without request id"
			),
			Broadcast { message } => {
				self.broadcast_messages.push_back(message);
			}
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
			success_messages: Default::default(),
			error_messages: Default::default(),
			broadcast_messages: Default::default(),
		}
	}
}
