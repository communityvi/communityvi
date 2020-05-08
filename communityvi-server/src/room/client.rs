use crate::connection::sender::MessageSender;
use crate::message::broadcast::Broadcast;
use crate::message::server_response::ServerResponseWithId;
use crate::room::client_id::ClientId;
use crate::room::Room;
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
	pub room: Room,
}

impl Client {
	pub fn new(id: ClientId, name: String, connection: MessageSender, room: Room) -> Self {
		Self {
			inner: Arc::new(Inner {
				id,
				name,
				connection,
				room,
			}),
		}
	}

	pub fn id(&self) -> ClientId {
		self.inner.id
	}

	pub fn name(&self) -> &str {
		self.inner.name.as_str()
	}

	pub async fn send(&self, response: ServerResponseWithId) -> bool {
		if self.inner.connection.send_response(response).await.is_err() {
			info!(
				"Failed to send message to client with id {} because it went away.",
				self.inner.id
			);
			false
		} else {
			true
		}
	}

	pub async fn broadcast<IntoBroadcast>(&self, broadcast: IntoBroadcast) -> bool
	where
		IntoBroadcast: Into<Broadcast>,
	{
		if self
			.inner
			.connection
			.send_broadcast_message(broadcast.into())
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
}
