use crate::client::{Client, ClientId};
use crate::client_handle::ClientHandle;
use crate::client_id_sequence::ClientIdSequence;
use crate::connection::client::ClientConnection;
use crate::message::ServerResponse;
use crate::room::error::RoomError;
use crate::room::state::State;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use futures::FutureExt;
use log::info;
use std::time::Duration;

pub mod error;
mod state;

#[derive(Default)]
pub struct Room {
	client_id_sequence: ClientIdSequence,
	clients: DashMap<ClientId, Client>,
	state: State,
}

impl Room {
	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub fn add_client(&self, name: String, connection: ClientConnection) -> Result<ClientHandle, RoomError> {
		if name.trim().is_empty() {
			return Err(RoomError::EmptyClientName);
		}

		let client_id = self.client_id_sequence.next();
		let client = Client::new(client_id, name, connection);

		let client_entry = self.clients.entry(client_id);
		if let Entry::Occupied(_) = &client_entry {
			unreachable!("There must never be two clients with the same id!")
		}

		Ok(client_entry.or_insert(client).into())
	}

	pub async fn remove_client(&self, client_id: ClientId) {
		if let Some((_, client)) = self.clients.remove(&client_id) {
			info!("Removed client: {} {}", client.id(), client.name());
			self.broadcast(ServerResponse::Left {
				id: client.id(),
				name: client.name().to_string(),
			})
			.await;
		}
	}

	pub fn get_client_by_id(&self, client_id: ClientId) -> Option<ClientHandle> {
		self.clients.get(&client_id).map(ClientHandle::from)
	}

	pub async fn singlecast(&self, client: &Client, response: ServerResponse) -> Result<(), ()> {
		client.send(response).await
	}

	pub async fn broadcast(&self, response: ServerResponse) {
		let futures: Vec<_> = self
			.clients
			.iter()
			.map(move |client| {
				let response = response.clone();
				async move {
					let _ = client.send(response).await;
				}
			})
			.collect();
		futures::future::join_all(futures).map(|_: Vec<()>| ()).await
	}

	pub fn current_reference_time(&self) -> Duration {
		self.state.current_reference_time()
	}
}
