use crate::connection::sender::MessageSender;
use crate::message::outgoing::broadcast_message::{BroadcastMessage, ChatBroadcast};
use crate::reference_time::ReferenceTimer;
use crate::room::client::Client;
use crate::room::client_id::ClientId;
use crate::room::clients::Clients;
use crate::room::error::RoomError;
use crate::room::medium::{Medium, VersionedMedium};
use crate::user::UserRepository;
use chrono::Duration;
use js_int::{uint, UInt};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub mod client;
pub mod client_id;
mod client_id_sequence;
pub mod clients;
pub mod error;
pub mod medium;

#[derive(Clone)]
pub struct Room {
	inner: Arc<Inner>,
}

struct Inner {
	user_repository: Mutex<UserRepository>,
	clients: RwLock<Clients>,
	medium: Mutex<VersionedMedium>,
	reference_timer: ReferenceTimer,
	message_counters: Mutex<MessageCounters>,
}

#[derive(Default)]
struct MessageCounters {
	chat_message_counter: UInt,
	broadcast_counter: usize,
}

impl MessageCounters {
	pub fn fetch_and_increment_chat_counter(&mut self) -> UInt {
		let count = self.chat_message_counter;
		self.chat_message_counter += uint!(1);
		count
	}

	pub fn fetch_and_increment_broadcast_counter(&mut self) -> usize {
		let count = self.broadcast_counter;
		self.broadcast_counter += 1;
		count
	}
}

impl Room {
	pub fn new(reference_timer: ReferenceTimer, room_size_limit: usize) -> Self {
		let inner = Inner {
			user_repository: Mutex::default(),
			clients: RwLock::new(Clients::with_limit(room_size_limit)),
			medium: Mutex::default(),
			reference_timer,
			message_counters: Default::default(),
		};
		Self { inner: Arc::new(inner) }
	}

	/// Add a new client to the room, passing in a sender for sending messages to it.
	/// Returns the newly added client and a list of clients that had existed prior to adding this one.
	pub fn add_client_and_return_existing(
		&self,
		name: &str,
		message_sender: MessageSender,
	) -> Result<(Client, Vec<Client>), RoomError> {
		let user = self.inner.user_repository.lock().create_user(name)?;
		self.inner.clients.write().add_and_return_existing(user, message_sender)
	}

	pub fn remove_client(&self, client_id: ClientId) {
		let mut clients = self.inner.clients.write();

		if let Some(client) = clients.remove(client_id) {
			self.inner.user_repository.lock().remove(client.user());
		}

		if clients.is_empty() {
			self.eject_medium();
		}
	}

	pub fn send_chat_message(&self, sender: &Client, message: String) {
		let chat_counter = self.inner.message_counters.lock().fetch_and_increment_chat_counter();
		let chat_message = ChatBroadcast {
			sender_id: sender.id(),
			sender_name: sender.name().to_string(),
			message,
			counter: chat_counter,
		};
		self.broadcast(chat_message);
	}

	pub fn broadcast(&self, response: impl Into<BroadcastMessage> + Clone) {
		let message = response.into();
		let count = self
			.inner
			.message_counters
			.lock()
			.fetch_and_increment_broadcast_counter();
		let clients = self.inner.clients.read();
		for client in clients.iter_clients() {
			client.enqueue_broadcast(message.clone(), count);
		}
	}

	/// Insert a medium based on `previous_version`. If `previous_version` is too low, nothing happens
	/// and `None` is returned. This is similar to compare and swap.
	#[must_use]
	pub fn insert_medium(&self, medium: impl Into<Medium>, previous_version: UInt) -> Option<VersionedMedium> {
		let mut versioned_medium = self.inner.medium.lock();
		if previous_version != versioned_medium.version {
			return None;
		}

		versioned_medium.update(medium.into());

		Some(versioned_medium.clone())
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn play_medium(&self, start_time: Duration, previous_version: UInt) -> Option<VersionedMedium> {
		let reference_now = Duration::from_std(self.inner.reference_timer.reference_time())
			.expect("This won't happen unless you run the server for more than 9_223_372_036_854_775_807 seconds :)");
		self.inner
			.medium
			.lock()
			.play(start_time, reference_now, previous_version)
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn pause_medium(&self, at_position: Duration, previous_version: UInt) -> Option<VersionedMedium> {
		self.inner.medium.lock().pause(at_position, previous_version)
	}

	fn eject_medium(&self) {
		self.inner.medium.lock().update(Medium::Empty);
	}

	pub fn medium(&self) -> VersionedMedium {
		self.inner.medium.lock().clone()
	}
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod test {
	use super::*;
	use crate::room::medium::fixed_length::FixedLengthMedium;
	use crate::utils::fake_message_sender::FakeMessageSender;
	use chrono::Duration;
	use js_int::uint;

	#[test]
	fn should_not_allow_adding_more_clients_than_room_size() {
		let room = Room::new(ReferenceTimer::default(), 2);
		for count in 1..=2 {
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = room.add_client_and_return_existing(&format!("{count}"), message_sender.clone()) {
				panic!("Failed to add client {count}: {error}");
			}
		}

		let message_sender = MessageSender::from(FakeMessageSender::default());
		let result = room.add_client_and_return_existing("elephant", message_sender);
		assert!(matches!(result, Err(RoomError::RoomFull)));
	}

	#[test]
	fn should_eject_the_inserted_medium_once_all_clients_have_left_the_room() {
		let room = Room::new(ReferenceTimer::default(), 10);
		let name = "牧瀬 紅莉栖";

		let message_sender = MessageSender::from(FakeMessageSender::default());
		let (makise_kurisu, _) = room
			.add_client_and_return_existing(name, message_sender)
			.expect("Failed to add client with same name after first is gone");
		let medium = FixedLengthMedium::new("愛のむきだし".to_string(), Duration::minutes(237));
		room.insert_medium(medium, uint!(0)).expect("Failed to insert medium");

		room.remove_client(makise_kurisu.id());
		assert_eq!(
			room.medium(),
			VersionedMedium {
				medium: Medium::Empty,
				version: uint!(2),
			},
			"A medium was still left in the room!"
		);
	}

	#[test]
	fn should_not_insert_medium_with_smaller_previous_version() {
		let room = Room::new(ReferenceTimer::default(), 1);
		room.insert_medium(Medium::Empty, uint!(0))
			.expect("Failed to insert medium"); // increase the version
		assert_eq!(room.medium().version, uint!(1));

		assert!(
			room.insert_medium(Medium::Empty, uint!(0)).is_none(),
			"Must not be able to insert"
		);
		assert_eq!(room.medium().version, uint!(1));
	}

	#[test]
	fn should_not_insert_medium_with_larger_previous_version() {
		let room = Room::new(ReferenceTimer::default(), 1);
		assert!(
			room.insert_medium(Medium::Empty, uint!(1)).is_none(),
			"Must not be able to insert"
		);
		assert_eq!(room.medium().version, uint!(0));
	}

	#[test]
	fn add_client_should_return_list_of_existing_clients() {
		let room = Room::new(ReferenceTimer::default(), 10);
		let jake_sender = FakeMessageSender::default();
		let (jake, existing_clients) = room.add_client_and_return_existing("Jake", jake_sender.into()).unwrap();
		assert!(existing_clients.is_empty());

		let elwood_sender = FakeMessageSender::default();
		let (_, existing_clients) = room
			.add_client_and_return_existing("Elwood", elwood_sender.into())
			.unwrap();
		assert_eq!(existing_clients.len(), 1);
		let existing_jake = &existing_clients[0];
		assert_eq!(jake.id(), existing_jake.id());
		assert_eq!(jake.name(), existing_jake.name());
	}
}
