use crate::message::{ClientRequest, Message, ServerResponse};
use crate::state::PlaybackState::{self, *};
use contrie::ConSet;
use futures::channel::mpsc::{SendError, Sender};
use futures::FutureExt;
use futures::SinkExt;
use log::info;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

pub struct Room {
	playback_state: Mutex<PlaybackState>,
	next_client_id: AtomicUsize,
	next_sequence_number: AtomicU64,
	pub clients: Arc<ConSet<Client>>,
}

impl Default for Room {
	fn default() -> Self {
		Room {
			playback_state: Mutex::default(),
			next_client_id: AtomicUsize::new(0),
			next_sequence_number: AtomicU64::new(0),
			clients: Arc::new(ConSet::new()),
		}
	}
}

impl Room {
	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub fn add_client(&self, response_sender: Sender<Message<ServerResponse>>) -> Client {
		let id = self.next_client_id.fetch_add(1, Ordering::SeqCst);
		let client = Client {
			id,
			sender: response_sender,
		};
		let existing_client = self.clients.insert(client.clone());
		if existing_client != None {
			unreachable!("There must never be two clients with the same id!")
		}
		client
	}

	pub async fn singlecast(&self, client: &Client, response: ServerResponse) {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let message = Message {
			number,
			message: response,
		};
		self.send(client, message).await
	}

	pub async fn broadcast(&self, response: ServerResponse) {
		let number = self.next_sequence_number.fetch_add(1, Ordering::SeqCst);
		let message = Message {
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

	async fn send(&self, client: &Client, message: Message<ServerResponse>) {
		let _ = client.send(message).await.map_err(|error| {
			// Send errors happen when clients go away, so remove it from the list of clients and ignore the error
			self.clients.remove(&client.clone());
			info!("Client with id {} has gone away.", client.id());
		});
	}

	pub fn play(&self) {
		let mut playback_state = self.playback_state.lock();
		*playback_state = match *playback_state.deref() {
			Paused { position } => {
				let now = Instant::now();
				let start = now - position;
				Playing { start }
			}
			state @ _ => state, // TODO: Maybe error handling in PlaybackState::Empty case?
		}
	}

	pub fn pause(&self) {
		let mut playback_state = self.playback_state.lock();
		*playback_state = match *playback_state.deref() {
			Playing { start } => {
				let now = Instant::now();
				let position = now - start;
				Paused { position }
			}
			state @ _ => state, // TODO: Maybe error handling in PlaybackState::Empty case?
		}
	}

	pub fn skip_by(&self, offset: Duration) {
		// TODO: Ensure this doesn't skip past or before the video.
		// TODO: Somehow ensure there is no overflow
		let mut playback_state = self.playback_state.lock();
		*playback_state = match *playback_state.deref() {
			Playing { start } => {
				let new_start = start - offset;
				Playing { start: new_start }
			}
			Paused { position } => Paused {
				position: position + offset,
			},
			Empty => Empty, // TODO: Maybe error handling in this case?
		}
	}

	pub fn skip_to(&self, position: Duration) {
		// TODO: Ensure this doesn't skip past or before the video.
		// TODO: Somehow ensure there is no overflow
		let mut playback_state = self.playback_state.lock();
		*playback_state = match *playback_state.deref() {
			Playing { start } => {
				let now = Instant::now();
				let new_start = now - position;
				Playing { start: new_start }
			}
			Paused { position } => Paused { position },
			Empty => Empty, // TODO: Maybe error handling in this case.
		}
	}
}

#[derive(Clone, Debug)]
pub struct Client {
	id: usize,
	sender: Sender<Message<ServerResponse>>,
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
	pub fn id(&self) -> usize {
		self.id
	}

	async fn send(&self, message: Message<ServerResponse>) -> Result<(), ClientSendError> {
		let send_result = self.sender.clone().send(message).await;
		send_result.map_err(|_: SendError| self.clone().into())
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
