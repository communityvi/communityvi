use crate::message::{Message, OrderedMessage};
use contrie::ConSet;
use futures01::future::join_all;
use futures01::sync::mpsc::{SendError, Sender};
use futures01::{Future, Sink};
use log::info;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use futures::compat::Future01CompatExt;

pub struct Room {
	pub offset: AtomicI64,
	next_client_id: AtomicUsize,
	next_sequence_number: AtomicU64,
	pub clients: Arc<ConSet<Client>>,
}

impl Default for Room {
	fn default() -> Self {
		Room {
			offset: AtomicI64::new(0),
			next_client_id: AtomicUsize::new(0),
			next_sequence_number: AtomicU64::new(0),
			clients: Arc::new(ConSet::new()),
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

	pub fn singlecast(&self, client: &Client, message: Message) -> impl std::future::Future<Output = ()> {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let ordered_message = OrderedMessage { number, message };
		let clients = self.clients.clone();
		let client = client.clone();
		async move {
			let _ = client.send_async(ordered_message).await.map_err(|error| {
				// Send errors happen when clients go away, so remove it from the list of clients and ignore the error
				clients.remove(&error.client);
				info!("Client with id {} has gone away.", error.client.id());
			});
		}
	}

	pub fn broadcast(&self, message: Message) -> impl Future<Item = (), Error = Infallible> {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let ordered_message = OrderedMessage { number, message };
		let clients = self.clients.clone();
		let futures: Vec<_> = self
			.clients
			.iter()
			.map(move |client| {
				let clients = clients.clone();
				client.send(ordered_message.clone()).then(move |result| {
					let _ = result.map_err(|error| {
						// Send errors happen when clients go away, so remove it from the list of clients and ignore the error
						clients.remove(&error.client);
						info!("Client with id {} has gone away.", error.client.id());
					});
					Ok(())
				})
			})
			.collect();
		join_all(futures).map(|_results: Vec<()>| ())
	}
}

#[derive(Clone, Debug)]
pub struct Client {
	id: usize,
	sender: Sender<OrderedMessage>,
}

#[derive(Debug)]
struct ClientSendError {
	pub client: Client,
}

impl Display for ClientSendError {
	fn fmt(&self, formatter: &mut Formatter) -> Result<(), std::fmt::Error> {
		write!(formatter, "Failed to send message to client: {}", self.client.id())
	}
}

impl From<Client> for ClientSendError {
	fn from(client: Client) -> Self {
		ClientSendError { client }
	}
}

impl Error for ClientSendError {}

impl Client {
	pub(self) fn id(&self) -> usize {
		self.id
	}

	pub(self) fn send(&self, message: OrderedMessage) -> impl Future<Item = (), Error = ClientSendError> {
		let client = self.clone();
		self.sender
			.clone()
			.send(message)
			.map(|_| ())
			.map_err(move |_send_error: SendError<OrderedMessage>| client.into())
	}

	async fn send_async(&self, message: OrderedMessage) -> Result<(), ClientSendError> {
		let client = self.clone();
		let send_result = self.sender.clone().send(message).compat().await;
		send_result
			.map(|_: Sender<_>| ())
			.map_err(|_: SendError<_>| client.into())
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
