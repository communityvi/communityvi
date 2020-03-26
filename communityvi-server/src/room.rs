use crate::client::Client;
use crate::message::{ClientRequest, OrderedMessage, ServerResponse};
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
	pub fn add_client(&self, response_sender: Sender<OrderedMessage<ServerResponse>>) -> Client {
		let id = self.next_client_id.fetch_add(1, Ordering::SeqCst);
		let client = Client::new(id.into(), response_sender);
		let existing_client = self.clients.insert(client.clone());
		if existing_client != None {
			unreachable!("There must never be two clients with the same id!")
		}
		client
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
