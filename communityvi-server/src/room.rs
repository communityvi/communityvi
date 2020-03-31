use crate::atomic_sequence::AtomicSequence;
use crate::client::{Client, ClientId};
use crate::client_handle::ClientHandle;
use crate::client_id_sequence::ClientIdSequence;
use crate::connection::ClientConnection;
use crate::message::{OrderedMessage, ServerResponse};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use futures::FutureExt;
use log::info;

#[derive(Default)]
pub struct Room {
	client_id_sequence: ClientIdSequence,
	message_number_sequence: AtomicSequence,
	clients: DashMap<ClientId, Client>,
}

impl Room {
	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub fn add_client(&self, name: String, connection: ClientConnection) -> ClientHandle {
		let client_id = self.client_id_sequence.next();
		let client = Client::new(client_id, name, connection);

		let client_entry = self.clients.entry(client_id);
		if let Entry::Occupied(_) = &client_entry {
			unreachable!("There must never be two clients with the same id!")
		}
		client_entry.or_insert(client).into()
	}

	pub fn remove_client(&self, client_id: ClientId) {
		self.clients.remove(&client_id).map(|(_, client)| {
			info!("Removed client: {} {}", client.id(), client.name());
		});
	}

	pub fn get_client_by_id(&self, client_id: ClientId) -> Option<ClientHandle> {
		self.clients.get(&client_id).map(ClientHandle::from)
	}

	pub async fn singlecast(&self, client: &Client, response: ServerResponse) -> Result<(), ()> {
		let number = self.message_number_sequence.next();
		let message = OrderedMessage {
			number,
			message: response,
		};
		client.send(message).await
	}

	pub async fn broadcast(&self, response: ServerResponse) {
		let number = self.message_number_sequence.next();
		let message = OrderedMessage {
			number,
			message: response,
		};
		let futures: Vec<_> = self
			.clients
			.iter()
			.map(move |client| {
				let message = message.clone();
				async move {
					let _ = client.send(message).await;
				}
			})
			.collect();
		futures::future::join_all(futures).map(|_: Vec<()>| ()).await
	}
}
