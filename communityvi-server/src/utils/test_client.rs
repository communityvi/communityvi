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
use crate::utils::websocket_message_conversion::{
	rweb_websocket_message_to_tungstenite_message, tungstenite_message_to_rweb_websocket_message,
};
use anyhow::anyhow;
use async_trait::async_trait;
use futures_channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use js_int::UInt;
use rweb::test::WsClient;
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;

pub struct WebsocketTestClient {
	websocket_client: Box<dyn WebSocketClient>,
	success_messages: BTreeMap<UInt, SuccessMessage>,
	error_messages: BTreeMap<Option<UInt>, ErrorMessage>,
	broadcast_messages: VecDeque<BroadcastMessage>,
}

impl WebsocketTestClient {
	pub fn new() -> (MessageSender, MessageReceiver, Self) {
		let (client_sender, server_receiver) = mpsc::unbounded();
		let (server_sender, client_receiver) = mpsc::unbounded();

		let test_client = Self::from((client_sender, client_receiver));

		let message_sender = MessageSender::from(server_sender.sink_map_err(Into::into));
		let message_receiver = MessageReceiver::new(server_receiver.map(Result::Ok), message_sender.clone());

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
		self.websocket_client
			.send(message)
			.await
			.expect("Failed to send message via TestClient.");
	}

	pub async fn receive_raw(&mut self) -> WebSocketMessage {
		timeout(Duration::from_secs(1), self.websocket_client.receive())
			.await
			.expect("Timed out waiting for raw message.")
			.expect("Failed to receive message via TestClient")
	}

	pub async fn send_request(&mut self, request: impl Into<ClientRequest>) -> UInt {
		let request_id = rand::random::<u32>().into();
		self.send_request_with_id(request, request_id).await;
		request_id
	}

	pub async fn send_request_with_id(&mut self, request: impl Into<ClientRequest>, request_id: UInt) {
		let client_request = ClientRequestWithId {
			request_id,
			request: request.into(),
		};
		let websocket_message = WebSocketMessage::from(&client_request);
		self.send_raw(websocket_message).await;
	}

	pub async fn receive_success_message(&mut self, expected_request_id: UInt) -> SuccessMessage {
		loop {
			self.receive_outgoing_message().await;
			if let Some(success) = self.success_messages.remove(&expected_request_id) {
				return success;
			}
		}
	}

	pub async fn receive_error_message(&mut self, expected_request_id: Option<UInt>) -> ErrorMessage {
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

impl<Client> From<Client> for WebsocketTestClient
where
	Client: WebSocketClient + 'static,
{
	fn from(client: Client) -> Self {
		Self {
			websocket_client: Box::new(client),
			success_messages: Default::default(),
			error_messages: Default::default(),
			broadcast_messages: Default::default(),
		}
	}
}

#[async_trait]
pub trait WebSocketClient {
	async fn send(&mut self, message: WebSocketMessage) -> anyhow::Result<()>;
	async fn receive(&mut self) -> anyhow::Result<WebSocketMessage>;
}

#[async_trait]
impl WebSocketClient for WsClient {
	async fn send(&mut self, message: WebSocketMessage) -> anyhow::Result<()> {
		let rweb_message = tungstenite_message_to_rweb_websocket_message(message);
		self.send(rweb_message).await;
		Ok(())
	}

	async fn receive(&mut self) -> anyhow::Result<WebSocketMessage> {
		let rweb_message = self.recv().await?;
		Ok(rweb_websocket_message_to_tungstenite_message(rweb_message))
	}
}

#[async_trait]
impl WebSocketClient
	for (
		mpsc::UnboundedSender<WebSocketMessage>,
		mpsc::UnboundedReceiver<WebSocketMessage>,
	)
{
	async fn send(&mut self, message: WebSocketMessage) -> anyhow::Result<()> {
		let (sender, _) = self;
		Ok(sender.send(message).await?)
	}

	async fn receive(&mut self) -> anyhow::Result<WebSocketMessage> {
		let (_, receiver) = self;
		receiver
			.next()
			.await
			.ok_or_else(|| anyhow!("Failed to receive message"))
	}
}

#[async_trait]
impl<Socket> WebSocketClient for WebSocketStream<Socket>
where
	Socket: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
	async fn send(&mut self, message: WebSocketMessage) -> anyhow::Result<()> {
		Ok(SinkExt::send(self, message).await?)
	}

	async fn receive(&mut self) -> anyhow::Result<WebSocketMessage> {
		match StreamExt::next(self).await {
			Some(Ok(message)) => Ok(message),
			Some(Err(error)) => Err(error.into()),
			None => Err(anyhow!("Failed to receive message")),
		}
	}
}
