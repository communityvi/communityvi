use crate::connection::Connection;
use crate::connection::broadcast_buffer::BroadcastBuffer;
use crate::connection::sender::MessageSender;
use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use crate::room::session_id::SessionId;
use crate::user::User;
use js_int::UInt;
use log::info;
use std::sync::Arc;

#[derive(Clone)]
pub struct Client {
	inner: Arc<Inner>,
}

struct Inner {
	id: SessionId,
	user: User,
	connection: Connection,
}

impl Client {
	pub fn new(id: SessionId, user: User, broadcast_buffer: BroadcastBuffer, sender: MessageSender) -> Self {
		let connection = Connection::new(sender, broadcast_buffer);
		Self {
			inner: Arc::new(Inner { id, user, connection }),
		}
	}

	pub fn id(&self) -> SessionId {
		self.inner.id
	}

	pub fn name(&self) -> &str {
		self.inner.user.name()
	}

	pub fn user(&self) -> &User {
		&self.inner.user
	}

	pub async fn send_success_message(&self, message: SuccessMessage, request_id: UInt) -> bool {
		let success = self.inner.connection.send_success_message(message, request_id).await;
		if !success {
			info!(
				"Failed to send success message to client with id {} because it went away.",
				self.inner.id
			);
		}
		success
	}

	pub async fn send_error_message(&self, message: ErrorMessage, request_id: Option<UInt>) -> bool {
		let success = self.inner.connection.send_error_message(message, request_id).await;
		if !success {
			info!(
				"Failed to send error message to client with id {} because it went away.",
				self.inner.id
			);
		}
		success
	}

	pub async fn send_broadcast_message(&self, message: impl Into<BroadcastMessage> + Unpin) -> bool {
		let success = self.inner.connection.send_broadcast_message(message.into()).await;
		if !success {
			info!(
				"Failed to send broadcast to client with id {} because it went away.",
				self.inner.id
			);
		}
		success
	}

	pub fn enqueue_broadcast(&self, message: impl Into<BroadcastMessage> + Unpin, count: usize) {
		self.inner.connection.enqueue_broadcast(message.into(), count);
	}

	pub async fn wait_for_broadcast(&self) -> BroadcastMessage {
		self.inner.connection.wait_for_broadcast().await
	}

	pub async fn send_ping(&self, payload: Vec<u8>) -> bool {
		let success = self.inner.connection.send_ping(payload).await;
		if !success {
			info!(
				"Failed to send ping to client with id {} because it went away.",
				self.inner.id
			);
		}
		success
	}
}
