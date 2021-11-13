use crate::connection::broadcast_buffer::BroadcastBuffer;
use crate::connection::sender::MessageSender;
use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use crate::room::client_id::ClientId;
use js_int::UInt;
use log::info;
use std::sync::Arc;

#[derive(Clone)]
pub struct Client {
	inner: Arc<Inner>,
}

struct Inner {
	pub id: ClientId,
	pub name: String,
	pub connection: MessageSender,
	pub broadcast_buffer: BroadcastBuffer,
}

impl Client {
	pub fn new(id: ClientId, name: String, broadcast_buffer: BroadcastBuffer, connection: MessageSender) -> Self {
		Self {
			inner: Arc::new(Inner {
				id,
				name,
				connection,
				broadcast_buffer,
			}),
		}
	}

	pub fn id(&self) -> ClientId {
		self.inner.id
	}

	pub fn name(&self) -> &str {
		self.inner.name.as_str()
	}

	pub async fn send_success_message(&self, message: SuccessMessage, request_id: UInt) -> bool {
		if self
			.inner
			.connection
			.send_success_message(message, request_id)
			.await
			.is_err()
		{
			info!(
				"Failed to send success message to client with id {} because it went away.",
				self.inner.id
			);
			false
		} else {
			true
		}
	}

	pub async fn send_error_message(&self, message: ErrorMessage, request_id: Option<UInt>) -> bool {
		if self
			.inner
			.connection
			.send_error_message(message, request_id)
			.await
			.is_err()
		{
			info!(
				"Failed to send error message to client with id {} because it went away.",
				self.inner.id
			);
			false
		} else {
			true
		}
	}

	pub async fn send_broadcast_message(&self, message: impl Into<BroadcastMessage> + Unpin) -> bool {
		if self
			.inner
			.connection
			.send_broadcast_message(message.into())
			.await
			.is_err()
		{
			info!(
				"Failed to send broadcast to client with id {} because it went away.",
				self.inner.id
			);
			false
		} else {
			true
		}
	}

	pub fn enqueue_broadcast(&self, message: impl Into<BroadcastMessage> + Unpin, count: usize) {
		self.inner.broadcast_buffer.enqueue(message.into(), count);
	}

	pub async fn wait_for_broadcast(&self) -> BroadcastMessage {
		self.inner.broadcast_buffer.wait_for_broadcast().await
	}

	pub async fn send_ping(&self, payload: Vec<u8>) -> bool {
		if self.inner.connection.send_ping(payload).await.is_err() {
			info!(
				"Failed to send ping to client with id {} because it went away.",
				self.inner.id
			);
			false
		} else {
			true
		}
	}
}
