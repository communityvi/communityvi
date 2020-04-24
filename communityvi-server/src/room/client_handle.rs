use crate::message::ServerResponse;
use crate::room::client::ClientId;
use crate::room::client_reference::ClientReference;
use crate::room::Room;

use log::info;
use std::ops::Deref;
use std::sync::Arc;

/// Handle to a client. It is bound to the lifecycle of a client.
/// It is constructed when adding a `Client` to a `Room` and dropping this
/// will remove the `Client` from the room.
pub struct ClientHandle {
	maybe_handle: MaybeClientHandle,
}

/// Handle to a client that may or may not be there. It is not bound to the
/// lifecycle of a client.
pub struct MaybeClientHandle {
	client_id: ClientId,
	room: Arc<Room>,
}

impl MaybeClientHandle {
	pub fn new(client_id: ClientId, room: Arc<Room>) -> Self {
		Self { client_id, room }
	}

	pub fn name(&self) -> Option<String> {
		self.client_reference().map(|client| client.name.clone())
	}

	pub fn id(&self) -> ClientId {
		self.client_id
	}

	pub async fn send(&self, response: ServerResponse) -> bool {
		let connection = if let Some(client_reference) = self.client_reference() {
			client_reference.connection.clone()
		} else {
			info!(
				"Failed to send message to Client with id {} because it doesn't exist (anymore?).",
				self.client_id
			);
			return false;
		};

		if connection.send(response).await.is_err() {
			info!(
				"Failed to send message to client with id {} because it went away.",
				self.client_id
			);
			false
		} else {
			true
		}
	}

	fn client_reference(&self) -> Option<ClientReference> {
		self.room.client_reference_by_id(self.client_id)
	}
}

impl ClientHandle {
	/// Construct a new client handle from a `Room` and `ClientId`.
	/// IMPORTANT: This must only be constructed by a room, when creating a new client.
	pub(super) fn new(client_id: ClientId, room: Arc<Room>) -> Self {
		Self {
			maybe_handle: MaybeClientHandle::new(client_id, room),
		}
	}

	/// Get the name of the client. Since this is coupled to the lifecycle of the actual client,
	/// this lookup will always succeed.
	pub fn name(&self) -> String {
		self.maybe_handle
			.name()
			.expect("Encountered a ClientHandle with missing client.")
	}
}

impl Deref for ClientHandle {
	type Target = MaybeClientHandle;

	fn deref(&self) -> &Self::Target {
		&self.maybe_handle
	}
}

impl Drop for ClientHandle {
	fn drop(&mut self) {
		if !self.room.remove_client(self.client_id) {
			unreachable!(
				"Failed to remove client id {} when dropping client handle.",
				self.client_id
			)
		}
	}
}
