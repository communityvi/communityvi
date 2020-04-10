use crate::message::{OrderedMessage, ServerResponse, WebSocketMessage};
use futures::stream::SplitSink;
use futures::Sink;
use futures::SinkExt;
use std::fmt::Debug;
use std::ops::Range;
use std::sync::Arc;
use warp::ws::WebSocket;

#[derive(Debug)]
pub struct ClientConnection<ResponseSink = SplitSink<WebSocket, WebSocketMessage>> {
	inner: Arc<tokio::sync::Mutex<ClientConnectionInner<ResponseSink>>>,
}

impl<T> Clone for ClientConnection<T> {
	fn clone(&self) -> Self {
		ClientConnection {
			inner: self.inner.clone(),
		}
	}
}

#[derive(Debug)]
struct ClientConnectionInner<ResponseSink> {
	response_sink: ResponseSink,
	message_number_sequence: Range<u64>,
}

impl<ResponseSink> ClientConnection<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage> + Unpin,
{
	pub fn new(response_sink: ResponseSink) -> Self {
		let inner = ClientConnectionInner {
			response_sink,
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

		inner.response_sink.send(websocket_message).await.map_err(|_| ())
	}

	pub async fn close(&self) {
		let mut inner = self.inner.lock().await;
		let _ = inner.response_sink.send(WebSocketMessage::close()).await;
	}
}
