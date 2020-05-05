use crate::message::{server_response::ServerResponse, WebSocketMessage};
use crate::server::WebSocket;
use async_trait::async_trait;
use futures::stream::SplitSink;
use futures::Sink;
use futures::SinkExt;
use std::pin::Pin;
use std::sync::Arc;

pub type MessageSender = Pin<Arc<dyn MessageSenderTrait + Send + Sync>>;

#[async_trait]
pub trait MessageSenderTrait {
	async fn send(&self, message: ServerResponse) -> Result<(), ()>;
	async fn close(&self);
}

pub type WebSocketMessageSender = SinkMessageSender<SplitSink<WebSocket, WebSocketMessage>>;

pub struct SinkMessageSender<ResponseSink> {
	inner: tokio::sync::Mutex<SinkMessageSenderInner<ResponseSink>>,
}

struct SinkMessageSenderInner<ResponseSink> {
	response_sink: ResponseSink,
}

#[async_trait]
impl<ResponseSink> MessageSenderTrait for SinkMessageSender<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage> + Send + Unpin + 'static,
{
	async fn send(&self, response: ServerResponse) -> Result<(), ()> {
		let mut inner = self.inner.lock().await;

		let websocket_message = WebSocketMessage::from(&response);

		inner.response_sink.send(websocket_message).await.map_err(|_| ())
	}

	async fn close(&self) {
		let mut inner = self.inner.lock().await;
		let _ = inner.response_sink.send(WebSocketMessage::Close(None)).await;
	}
}

impl<ResponseSink> SinkMessageSender<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage>,
{
	pub fn new(response_sink: ResponseSink) -> Self {
		let inner = SinkMessageSenderInner { response_sink };
		let connection = Self { inner: inner.into() };
		connection
	}
}

impl<ResponseSink> From<SinkMessageSender<ResponseSink>> for MessageSender
where
	ResponseSink: Sink<WebSocketMessage> + Send + Unpin + 'static,
{
	fn from(sink_client_connection: SinkMessageSender<ResponseSink>) -> Self {
		Arc::pin(sink_client_connection)
	}
}
