use crate::connection::client::ClientConnection;
use crate::message::ServerResponse;
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
	pub connection: ClientConnection,
	pub room: Room,
}

impl Client {
	pub fn new(id: ClientId, name: String, connection: ClientConnection, room: Room) -> Self {
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

	pub async fn send(&self, response: ServerResponse) -> bool {
		if self.inner.connection.send(response).await.is_err() {
			info!(
				"Failed to send message to client with id {} because it went away.",
				self.inner.id
			);
			false
		} else {
			true
		}
	}
}
