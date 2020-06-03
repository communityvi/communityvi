use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use crate::message::outgoing::OutgoingMessage;
use crate::message::WebSocketMessage;
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
	async fn send_success_message(&self, message: SuccessMessage, request_id: u64) -> Result<(), ()>;
	async fn send_error_message(&self, message: ErrorMessage, request_id: Option<u64>) -> Result<(), ()>;
	async fn send_broadcast_message(&self, message: BroadcastMessage) -> Result<(), ()>;
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
	async fn send_success_message(&self, message: SuccessMessage, request_id: u64) -> Result<(), ()> {
		let outgoing_message = OutgoingMessage::Success { request_id, message };
		self.send_message(outgoing_message).await
	}

	async fn send_error_message(&self, message: ErrorMessage, request_id: Option<u64>) -> Result<(), ()> {
		let outgoing_message = OutgoingMessage::Error { request_id, message };
		self.send_message(outgoing_message).await
	}

	async fn send_broadcast_message(&self, message: BroadcastMessage) -> Result<(), ()> {
		let outgoing_message = OutgoingMessage::Broadcast { message };
		self.send_message(outgoing_message).await
	}

	async fn close(&self) {
		let mut inner = self.inner.lock().await;
		let _ = inner.response_sink.send(WebSocketMessage::Close(None)).await;
	}
}

impl<ResponseSink> SinkMessageSender<ResponseSink>
where
	ResponseSink: Sink<WebSocketMessage> + Unpin,
{
	pub fn new(response_sink: ResponseSink) -> Self {
		let inner = SinkMessageSenderInner { response_sink };
		Self { inner: inner.into() }
	}

	async fn send_message(&self, message: OutgoingMessage) -> Result<(), ()> {
		let mut inner = self.inner.lock().await;

		let websocket_message = WebSocketMessage::from(&message);

		inner.response_sink.send(websocket_message).await.map_err(|_| ())
	}
}

impl<ResponseSink> From<SinkMessageSender<ResponseSink>> for MessageSender
where
	ResponseSink: Sink<WebSocketMessage> + Send + Unpin + 'static,
{
	fn from(sink_message_sender: SinkMessageSender<ResponseSink>) -> Self {
		Arc::pin(sink_message_sender)
	}
}
