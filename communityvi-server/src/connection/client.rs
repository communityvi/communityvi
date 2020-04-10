use crate::message::{OrderedMessage, ServerResponse, WebSocketMessage};
use futures::stream::SplitSink;
use futures::SinkExt;
use std::ops::Range;
use std::sync::Arc;
use warp::ws::WebSocket;

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
	pub fn new(websocket_sink: SplitSink<WebSocket, WebSocketMessage>) -> Self {
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

	pub async fn close(&self) {
		let mut inner = self.inner.lock().await;
		let _ = inner.websocket_sink.send(WebSocketMessage::close()).await;
	}
}
