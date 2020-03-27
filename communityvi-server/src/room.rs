use crate::client::{Client, ClientId};
use crate::message::{OrderedMessage, ServerResponse};
use ahash::RandomState;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use futures::channel::mpsc::Sender;
use futures::FutureExt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Default)]
pub struct Room {
	next_client_id: AtomicUsize,
	next_sequence_number: AtomicU64,
	clients: DashMap<ClientId, Client>,
}

type ClientHandle<'a> = Ref<'a, ClientId, Client, RandomState>;

impl Room {
	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub fn add_client(&self, response_sender: Sender<OrderedMessage<ServerResponse>>) -> ClientId {
		let id = self.next_client_id.fetch_add(1, Ordering::SeqCst).into();
		let client = Client::new(id, response_sender);

		let existing_client = self.clients.insert(id, client);
		if existing_client.is_some() {
			unreachable!("There must never be two clients with the same id!")
		}

		id
	}

	pub fn remove_client(&self, client_id: ClientId) {
		self.clients.remove(&client_id);
	}

	pub fn get_client_by_id(&self, client_id: ClientId) -> Option<ClientHandle> {
		self.clients.get(&client_id)
	}

	pub async fn singlecast(&self, client: &Client, response: ServerResponse) {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let message = OrderedMessage {
			number,
			message: response,
		};
		self.send(client, message).await
	}

	pub async fn broadcast(&self, response: ServerResponse) {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let message = OrderedMessage {
			number,
			message: response,
		};
		let futures: Vec<_> = self
			.clients
			.iter()
			.map(move |client| {
				let message = message.clone();
				async move { self.send(&client, message).await }
			})
			.collect();
		futures::future::join_all(futures).map(|_: Vec<()>| ()).await
	}

	async fn send(&self, client: &Client, message: OrderedMessage<ServerResponse>) {
		let _ = client.send(message).await;
	}
}
