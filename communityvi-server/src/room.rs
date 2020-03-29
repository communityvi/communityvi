use crate::atomic_sequence::AtomicSequence;
use crate::client::{Client, ClientId};
use crate::client_id_sequence::ClientIdSequence;
use crate::message::{OrderedMessage, ServerResponse};
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use futures::channel::mpsc::Sender;
use futures::FutureExt;

#[derive(Default)]
pub struct Room {
	client_id_sequence: ClientIdSequence,
	message_number_sequence: AtomicSequence,
	clients: DashMap<ClientId, Client>,
}

type ClientHandle<'a> = Ref<'a, ClientId, Client>;

impl Room {
	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub async fn add_client(&self, response_sender: Sender<OrderedMessage<ServerResponse>>) -> Option<ClientId> {
		let client_id = self.client_id_sequence.next();
		let client = Client::new(client_id, response_sender);

		let hello_message = OrderedMessage {
			number: self.message_number_sequence.next(),
			message: ServerResponse::Hello { id: client_id },
		};
		match client.send(hello_message).await {
			Ok(()) => (),
			Err(()) => return None,
		}

		let existing_client = self.clients.insert(client_id, client);
		if existing_client.is_some() {
			unreachable!("There must never be two clients with the same id!")
		}

		Some(client_id)
	}

	pub fn remove_client(&self, client_id: ClientId) {
		self.clients.remove(&client_id);
	}

	pub fn get_client_by_id(&self, client_id: ClientId) -> Option<ClientHandle> {
		self.clients.get(&client_id)
	}

	pub async fn singlecast(&self, client: &Client, response: ServerResponse) {
		let number = self.message_number_sequence.next();
		let message = OrderedMessage {
			number,
			message: response,
		};
		self.send(client, message).await
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
				async move { self.send(&client, message).await }
			})
			.collect();
		futures::future::join_all(futures).map(|_: Vec<()>| ()).await
	}

	async fn send(&self, client: &Client, message: OrderedMessage<ServerResponse>) {
		let _ = client.send(message).await;
	}
}
