use crate::message::{OrderedMessage, ServerResponse, WebSocketMessage};
use crate::server::WebSocket;
use async_trait::async_trait;
use futures::stream::SplitSink;
use futures::Sink;
use futures::SinkExt;
use std::pin::Pin;
use std::sync::Arc;

pub type ClientConnection = Pin<Arc<dyn ClientConnectionTrait + Send + Sync>>;

#[async_trait]
pub trait ClientConnectionTrait {
	async fn send(&self, message: ServerResponse) -> Result<(), ()>;
	async fn close(&self);
}

pub type WebSocketClientConnection = SinkClientConnection<SplitSink<WebSocket, WebSocketMessage>>;

pub struct SinkClientConnection<ResponseSink> {
	inner: tokio::sync::Mutex<SinkClientConnectionInner<ResponseSink>>,
}

struct SinkClientConnectionInner<ResponseSink> {
	response_sink: ResponseSink,
}

#[async_trait]
impl<ResponseSink> ClientConnectionTrait for SinkClientConnection<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage> + Send + Unpin + 'static,
{
	async fn send(&self, message: ServerResponse) -> Result<(), ()> {
		let mut inner = self.inner.lock().await;

		let ordered_message = OrderedMessage { message };
		let websocket_message = WebSocketMessage::from(&ordered_message);

		inner.response_sink.send(websocket_message).await.map_err(|_| ())
	}

	async fn close(&self) {
		let mut inner = self.inner.lock().await;
		let _ = inner.response_sink.send(WebSocketMessage::Close(None)).await;
	}
}

impl<ResponseSink> SinkClientConnection<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage>,
{
	pub fn new(response_sink: ResponseSink) -> Self {
		let inner = SinkClientConnectionInner { response_sink };
		let connection = Self { inner: inner.into() };
		connection
	}
}

impl<ResponseSink> From<SinkClientConnection<ResponseSink>> for ClientConnection
where
	ResponseSink: Sink<WebSocketMessage> + Send + Unpin + 'static,
{
	fn from(sink_client_connection: SinkClientConnection<ResponseSink>) -> Self {
		Arc::pin(sink_client_connection)
	}
}
