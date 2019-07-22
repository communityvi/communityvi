use crate::message::{Message, OrderedMessage};
use contrie::ConSet;
use futures::future::join_all;
use futures::sync::mpsc::Sender;
use futures::{Future, Sink};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};

pub struct Room {
	pub offset: AtomicI64,
	next_client_id: AtomicUsize,
	next_sequence_number: AtomicU64,
	pub clients: ConSet<Client>,
}

impl Default for Room {
	fn default() -> Self {
		Room {
			offset: AtomicI64::new(0),
			next_client_id: AtomicUsize::new(0),
			next_sequence_number: AtomicU64::new(0),
			clients: ConSet::new(),
		}
	}
}

impl Room {
	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub fn add_client(&self, sender: Sender<OrderedMessage>) -> Client {
		let id = self.next_client_id.fetch_add(1, Ordering::SeqCst);
		let client = Client { id, sender };
		let existing_client = self.clients.insert(client.clone());
		if existing_client != None {
			unreachable!("There must never be two clients with the same id!")
		}
		client
	}

	pub fn singlecast(&self, client: &Client, message: Message) -> impl Future<Item = (), Error = ()> {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let ordered_message = OrderedMessage { number, message };
		client.send(ordered_message)
	}

	pub fn broadcast(&self, message: Message) -> impl Future<Item = (), Error = ()> {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let ordered_message = OrderedMessage { number, message };
		let futures: Vec<_> = self
			.clients
			.iter()
			.map(|client| client.send(ordered_message.clone()))
			.collect();
		join_all(futures).map(|_| ()).map_err(|_| ())
	}
}

#[derive(Clone)]
pub struct Client {
	id: usize,
	sender: Sender<OrderedMessage>,
}

impl Client {
	pub(self) fn send(&self, message: OrderedMessage) -> impl Future<Item = (), Error = ()> {
		self
			.sender
			.clone()
			.send(message)
			.map(|_| ())
			// Discarding the SendError is ok since the other end might have been legitimately dropped
			.map_err(|_send_error| ())
	}
}

impl Hash for Client {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state)
	}
}

impl PartialEq for Client {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Eq for Client {}
