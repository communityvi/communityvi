use crate::message::{OrderedMessage, ServerResponse, WebSocketMessage};
use async_trait::async_trait;
use futures::stream::SplitSink;
use futures::Sink;
use futures::SinkExt;
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;
use warp::ws::WebSocket;

pub type ClientConnection = Pin<Box<dyn ClientConnectionTrait + Send + Sync>>;

#[async_trait]
pub trait ClientConnectionTrait {
	async fn send(&self, message: ServerResponse) -> Result<(), ()>;
	async fn close(&self);
	fn clone(&self) -> ClientConnection;
}

pub type WebSocketClientConnection = SinkClientConnection<SplitSink<WebSocket, WebSocketMessage>>;

pub struct SinkClientConnection<ResponseSink> {
	inner: Arc<tokio::sync::Mutex<SinkClientConnectionInner<ResponseSink>>>,
}

impl<T> Clone for SinkClientConnection<T> {
	fn clone(&self) -> Self {
		SinkClientConnection {
			inner: self.inner.clone(),
		}
	}
}

struct SinkClientConnectionInner<ResponseSink> {
	response_sink: ResponseSink,
	message_number_sequence: Range<u64>,
}

#[async_trait]
impl<ResponseSink> ClientConnectionTrait for SinkClientConnection<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage> + Send + Unpin + 'static,
{
	async fn send(&self, message: ServerResponse) -> Result<(), ()> {
		let mut inner = self.inner.lock().await;

		let ordered_message = OrderedMessage {
			number: inner.message_number_sequence.next().expect("Out of message numbers"),
			message,
		};
		let websocket_message = WebSocketMessage::from(&ordered_message);

		inner.response_sink.send(websocket_message).await.map_err(|_| ())
	}

	async fn close(&self) {
		let mut inner = self.inner.lock().await;
		let _ = inner.response_sink.send(WebSocketMessage::close()).await;
	}

	fn clone(&self) -> ClientConnection {
		Clone::clone(self).into()
	}
}

impl<ResponseSink> SinkClientConnection<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage>,
{
	pub fn new(response_sink: ResponseSink) -> Self {
		let inner = SinkClientConnectionInner {
			response_sink,
			message_number_sequence: (0..std::u64::MAX),
		};
		let connection = Self {
			inner: Arc::new(inner.into()),
		};
		connection
	}
}

impl<ResponseSink> From<SinkClientConnection<ResponseSink>> for ClientConnection
where
	ResponseSink: Sink<WebSocketMessage> + Send + Unpin + 'static,
{
	fn from(sink_client_connection: SinkClientConnection<ResponseSink>) -> Self {
		Box::pin(sink_client_connection)
	}
}
