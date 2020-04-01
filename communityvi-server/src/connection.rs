use crate::message::{ClientRequest, OrderedMessage, ServerResponse, WebSocketMessage};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::error;
use std::ops::Range;
use std::sync::Arc;
use warp::filters::ws::WebSocket;

pub fn split_websocket(websocket: WebSocket) -> (ClientConnection, ServerConnection) {
	let (websocket_sink, websocket_stream) = websocket.split();
	let client_connection = ClientConnection::new(websocket_sink);
	let server_connection = ServerConnection::new(websocket_stream);
	(client_connection, server_connection)
}

pub struct ServerConnection {
	websocket_stream: SplitStream<WebSocket>,
}

impl ServerConnection {
	fn new(websocket_stream: SplitStream<WebSocket>) -> Self {
		Self { websocket_stream }
	}

	/// Receive a message from the client or None if the connection has been closed.
	pub async fn receive(&mut self) -> Option<OrderedMessage<ClientRequest>> {
		let websocket_message = match self.websocket_stream.next().await {
			Some(Ok(websocket_message)) => websocket_message,
			Some(Err(error)) => {
				error!("Error streaming websocket message: {}, result.", error);
				return None;
			}
			None => return None,
		};
		Some(OrderedMessage::from(websocket_message))
	}
}

#[derive(Clone, Debug)]
pub struct ClientConnection {
	inner: Arc<tokio::sync::Mutex<ClientConnectionInner>>,
}

#[derive(Debug)]
struct ClientConnectionInner {
	websocket_sink: SplitSink<WebSocket, WebSocketMessage>,
	message_number_sequence: Range<u64>,
}

impl ClientConnection {
	fn new(websocket_sink: SplitSink<WebSocket, WebSocketMessage>) -> Self {
		let inner = ClientConnectionInner {
			websocket_sink,
			message_number_sequence: (0..std::u64::MAX),
		};
		Self {
			inner: Arc::new(inner.into()),
		}
	}

	pub async fn send(&self, message: ServerResponse) -> Result<(), ()> {
		let mut inner = self.inner.lock().await;

		let ordered_message = OrderedMessage {
			number: inner.message_number_sequence.next().expect("Out of message numbers"),
			message,
		};
		let websocket_message = WebSocketMessage::from(&ordered_message);

		inner.websocket_sink.send(websocket_message).await.map_err(|_| ())
	}
}
