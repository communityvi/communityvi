use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use crate::message::outgoing::OutgoingMessage;
use crate::message::WebSocketMessage;
use futures_util::{Sink, SinkExt};
use js_int::UInt;
use log::error;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
pub struct MessageSender {
	sink: Pin<Arc<tokio::sync::Mutex<dyn Sink<WebSocketMessage, Error = anyhow::Error> + Unpin + Send>>>,
}

impl<WebSocketSink> From<WebSocketSink> for MessageSender
where
	WebSocketSink: Sink<WebSocketMessage, Error = anyhow::Error> + Unpin + Send + 'static,
{
	fn from(websocket_sink: WebSocketSink) -> Self {
		Self {
			sink: Arc::pin(tokio::sync::Mutex::new(websocket_sink)),
		}
	}
}

impl MessageSender {
	pub async fn send_success_message(&self, message: SuccessMessage, request_id: UInt) -> Result<(), ()> {
		let outgoing_message = OutgoingMessage::Success { request_id, message };
		self.send_message(outgoing_message).await
	}

	pub async fn send_error_message(&self, message: ErrorMessage, request_id: Option<UInt>) -> Result<(), ()> {
		let outgoing_message = OutgoingMessage::Error { request_id, message };
		self.send_message(outgoing_message).await
	}

	pub async fn send_broadcast_message(&self, message: BroadcastMessage) -> Result<(), ()> {
		let outgoing_message = OutgoingMessage::Broadcast { message };
		self.send_message(outgoing_message).await
	}

	async fn send_message(&self, message: OutgoingMessage) -> Result<(), ()> {
		let mut sink = self.sink.lock().await;

		let websocket_message = WebSocketMessage::from(&message);

		sink.send(websocket_message)
			.await
			.map_err(|error| error!("Error while sending message: {:?}", error))
	}

	pub async fn send_ping(&self, payload: Vec<u8>) -> Result<(), ()> {
		let mut sink = self.sink.lock().await;
		let ping = WebSocketMessage::Ping(payload.into());
		sink.send(ping)
			.await
			.map_err(|error| error!("Error while sending `ping`: {:?}", error))
	}

	pub async fn send_pong(&self, payload: Vec<u8>) -> Result<(), ()> {
		let mut sink = self.sink.lock().await;
		let pong = WebSocketMessage::Pong(payload.into());
		sink.send(pong)
			.await
			.map_err(|error| error!("Error while sending `pong`: {error:?}"))
	}

	#[allow(let_underscore_drop)] // Ignore Clippy here because we don't care about the result.
	pub async fn close(&self) {
		let mut sink = self.sink.lock().await;
		let _ = sink.send(WebSocketMessage::Close(None)).await;
	}
}
