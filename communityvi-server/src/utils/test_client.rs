use crate::connection::receiver::{MessageReceiver, StreamMessageReceiver};
use crate::connection::sender::{MessageSender, SinkMessageSender};
use crate::message::broadcast::Broadcast;
use crate::message::client_request::RequestConvertible;
use crate::message::server_response::ServerResponseWithId;
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

		let sink_client_connection = SinkMessageSender::new(server_sender);
		let message_sender = MessageSender::from(sink_client_connection);
		let stream_server_connection = StreamMessageReceiver::new(server_receiver, message_sender.clone());

		let message_receiver = MessageReceiver::from(stream_server_connection);

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

	pub async fn send_request<RequestWithoutId>(&mut self, request: RequestWithoutId) -> u64
	where
		RequestWithoutId: RequestConvertible,
	{
		let request_id = rand::random();
		self.send_request_with_id(request, request_id).await;
		request_id
	}

	pub async fn send_request_with_id<RequestWithoutId>(&mut self, request: RequestWithoutId, request_id: u64)
	where
		RequestWithoutId: RequestConvertible,
	{
		let websocket_message = WebSocketMessage::from(&request.into().with_id(request_id));
		self.send_raw(websocket_message).await
	}

	pub async fn receive_response(&mut self) -> ServerResponseWithId {
		let websocket_message = self.receive_raw().await;
		ServerResponseWithId::try_from(&websocket_message).expect("Failed to deserialize ServerResponse")
	}

	pub async fn receive_broadcast(&mut self) -> Broadcast {
		let websocket_message = self.receive_raw().await;
		Broadcast::try_from(&websocket_message).expect("Failed to deserialize Broadcast")
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
