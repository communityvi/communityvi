use crate::connection::sender::MessageSender;
use crate::message::outgoing::broadcast_message::{BroadcastMessage, ChatBroadcast};
use crate::room::client::Client;
use crate::room::client_id::ClientId;
use crate::room::client_id_sequence::ClientIdSequence;
use crate::room::error::RoomError;
use crate::room::medium::{Medium, VersionedMedium};
use chrono::Duration;
use futures::FutureExt;
use parking_lot::{Mutex, RwLock, RwLockWriteGuard};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::time::Instant;
use unicode_skeleton::UnicodeSkeleton;

pub mod client;
pub mod client_id;
mod client_id_sequence;
pub mod error;
pub mod medium;

#[derive(Clone)]
pub struct Room {
	inner: Arc<Inner>,
}

struct Inner {
	client_id_sequence: ClientIdSequence,
	client_names: RwLock<HashSet<String>>,
	clients: RwLock<HashMap<ClientId, Client>>,
	medium: Mutex<VersionedMedium>,
	start_of_reference_time: Instant,
	chat_message_count: AtomicU64,
	room_size_limit: usize,
}

impl Room {
	pub fn new(room_size_limit: usize) -> Self {
		let inner = Inner {
			client_id_sequence: Default::default(),
			client_names: Default::default(),
			clients: Default::default(),
			medium: Mutex::default(),
			start_of_reference_time: std::time::Instant::now(),
			chat_message_count: AtomicU64::new(0),
			room_size_limit,
		};
		Self { inner: Arc::new(inner) }
	}

	/// Add a new client to the room, passing in a sender for sending messages to it.
	/// Returns the newly added client and a list of clients that had existed prior to adding this one.
	pub fn add_client_and_return_existing(
		&self,
		name: String,
		message_sender: MessageSender,
	) -> Result<(Client, Vec<Client>), RoomError> {
		if name.trim().is_empty() {
			return Err(RoomError::EmptyClientName);
		}

		const MAX_NAME_LENGTH: usize = 256;
		if name.len() > MAX_NAME_LENGTH {
			return Err(RoomError::ClientNameTooLong);
		}

		let (mut clients, mut names) = self.lock_clients_and_names_for_writing();
		if clients.len() >= self.inner.room_size_limit {
			return Err(RoomError::RoomFull);
		}

		if !names.insert(normalized_name(name.as_str())) {
			return Err(RoomError::ClientNameAlreadyInUse);
		}

		let client_id = self.inner.client_id_sequence.next();
		let client = Client::new(client_id, name, message_sender);

		let existing_clients = clients.iter().map(|(_id, client)| client.clone()).collect();
		if clients.insert(client_id, client.clone()).is_some() {
			unreachable!("There must never be two clients with the same id!")
		}

		Ok((client, existing_clients))
	}

	/// Get a lock of the client and client names.
	/// Use this method to ensure the locks are always taken in the same order to prevent deadlock.
	fn lock_clients_and_names_for_writing(
		&self,
	) -> (
		RwLockWriteGuard<HashMap<ClientId, Client>>,
		RwLockWriteGuard<HashSet<String>>,
	) {
		let clients = self.inner.clients.write();
		let names = self.inner.client_names.write();
		(clients, names)
	}

	pub fn remove_client(&self, client_id: ClientId) -> bool {
		let (mut clients, mut names) = self.lock_clients_and_names_for_writing();
		clients
			.remove(&client_id)
			.map(|client| names.remove(&normalized_name(client.name())))
			.map(|_client_name| {
				if clients.is_empty() {
					self.eject_medium();
				}
			})
			.is_some()
	}

	pub async fn send_chat_message(&self, sender: &Client, message: String) {
		let incremented_counter = self.inner.chat_message_count.fetch_add(1, SeqCst);
		let chat_message = ChatBroadcast {
			sender_id: sender.id(),
			sender_name: sender.name().to_string(),
			message,
			counter: incremented_counter,
		};
		self.broadcast(chat_message).await;
	}

	pub async fn broadcast(&self, response: impl Into<BroadcastMessage> + Clone) {
		let futures: Vec<_> = self
			.inner
			.clients
			.read()
			.iter()
			.map(|(_id, client)| client.clone())
			.map(move |client| {
				let response = response.clone();
				async move {
					client.send_broadcast_message(response).await;
				}
			})
			.collect();
		futures::future::join_all(futures).map(|_: Vec<()>| ()).await
	}

	pub fn current_reference_time(&self) -> std::time::Duration {
		self.inner.start_of_reference_time.elapsed()
	}

	/// Insert a medium based on `previous_version`. If `previous_version` is too low, nothing happens
	/// and `None` is returned. This is similar to compare and swap.
	#[must_use]
	pub fn insert_medium(&self, medium: impl Into<Medium>, previous_version: u64) -> Option<VersionedMedium> {
		let mut versioned_medium = self.inner.medium.lock();
		if previous_version != versioned_medium.version {
			return None;
		}

		versioned_medium.update(medium.into());

		Some(versioned_medium.clone())
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn play_medium(&self, start_time: Duration, previous_version: u64) -> Option<VersionedMedium> {
		let reference_now = Duration::from_std(self.current_reference_time())
			.expect("This won't happen unless you run the server for more than 9_223_372_036_854_775_807 seconds :)");
		self.inner
			.medium
			.lock()
			.play(start_time, reference_now, previous_version)
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn pause_medium(&self, at_position: Duration, previous_version: u64) -> Option<VersionedMedium> {
		self.inner.medium.lock().pause(at_position, previous_version)
	}

	fn eject_medium(&self) {
		self.inner.medium.lock().update(Medium::Empty)
	}

	pub fn medium(&self) -> VersionedMedium {
		self.inner.medium.lock().clone()
	}
}

/// This function makes sure that unicode characters get correctly decomposed,
/// normalized and some homograph attacks are hindered, disregarding whitespace.
fn normalized_name(name: &str) -> String {
	name.split_whitespace()
		.flat_map(UnicodeSkeleton::skeleton_chars)
		.collect()
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::room::medium::fixed_length::FixedLengthMedium;
	use crate::utils::fake_message_sender::FakeMessageSender;
	use chrono::Duration;

	#[test]
	fn should_not_add_client_with_empty_name() {
		let room = Room::new(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		let result = room.add_client_and_return_existing("".to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::EmptyClientName)));
	}

	#[test]
	fn should_not_add_client_with_blank_name() {
		let room = Room::new(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		let result = room.add_client_and_return_existing("  	 ".to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::EmptyClientName)));
	}

	#[test]
	fn should_normalize_unicode_strings() {
		assert_eq!(normalized_name("C\u{327}"), "C\u{326}");
		assert_eq!(normalized_name("é"), "e\u{301}");
		assert_eq!(normalized_name("\u{0C5}"), "A\u{30A}");
		assert_eq!(normalized_name("\u{212B}"), "A\u{30A}");
		assert_eq!(normalized_name("\u{391}"), "A");
		assert_eq!(normalized_name("\u{410}"), "A");
		assert_eq!(normalized_name("𝔭𝒶ỿ𝕡𝕒ℓ"), "paypal");
		assert_eq!(normalized_name("𝒶𝒷𝒸"), "abc");
		assert_eq!(normalized_name("ℝ𝓊𝓈𝓉"), "Rust");
		assert_eq!(normalized_name("аррӏе.com"), "appie.corn");
		assert_eq!(normalized_name("𝔭𝒶   ỿ𝕡𝕒		ℓ"), "paypal");
		assert_eq!(normalized_name("𝒶𝒷\r\n𝒸"), "abc");
		assert_eq!(normalized_name("ℝ		𝓊𝓈 𝓉"), "Rust");
		assert_eq!(normalized_name("арр    ӏе.	com"), "appie.corn");
	}

	#[test]
	#[ignore]
	fn should_prevent_whole_script_homographs() {
		/*
		 * "Our IDN threat model specifically excludes whole-script homographs, because they can't
		 *  be detected programmatically and our "TLD whitelist" approach didn't scale in the face
		 *  of a large number of new TLDs. If you are buying a domain in a registry which does not
		 *  have proper anti-spoofing protections (like .com), it is sadly the responsibility of
		 *  domain owners to check for whole-script homographs and register them."
		 *  - https://bugzilla.mozilla.org/show_bug.cgi?id=1332714#c5 by Gervase Markham, 2017-01-25
		 */
		assert_eq!(normalized_name("аррӏе.com"), normalized_name("apple.com"));
	}

	#[test]
	fn should_not_add_two_clients_with_the_same_name() {
		let room = Room::new(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		room.add_client_and_return_existing("Anorak  ".to_string(), message_sender.clone())
			.expect("First add did not succeed!");
		let result = room.add_client_and_return_existing("   Anorak".to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::ClientNameAlreadyInUse)));
	}

	#[test]
	fn should_allow_adding_client_with_the_same_name_after_first_has_been_removed() {
		let room = Room::new(10);
		let name = "牧瀬 紅莉栖";

		{
			let message_sender = MessageSender::from(FakeMessageSender::default());
			let (client, _) = room
				.add_client_and_return_existing(name.to_string(), message_sender.clone())
				.expect("Failed to add client");
			room.remove_client(client.id());
		}

		let message_sender = MessageSender::from(FakeMessageSender::default());
		room.add_client_and_return_existing(name.to_string(), message_sender.clone())
			.expect("Failed to add client with same name after first is gone");
	}

	#[test]
	fn should_allow_adding_client_with_name_no_longer_than_256_bytes() {
		let long_name = String::from_utf8(vec![0x41u8; 256]).unwrap();
		let room = Room::new(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		room.add_client_and_return_existing(long_name.to_string(), message_sender.clone())
			.expect("Failed to add client with name that is not too long");
	}

	#[test]
	fn should_not_allow_adding_client_with_name_longer_than_256_bytes() {
		let long_name = String::from_utf8(vec![0x41u8; 257]).unwrap();
		let room = Room::new(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		let result = room.add_client_and_return_existing(long_name.to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::ClientNameTooLong)));
	}

	#[test]
	fn should_allow_adding_clients_up_to_room_size_limit() {
		let room = Room::new(2);
		for count in 1..=2 {
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = room.add_client_and_return_existing(format!("{}", count), message_sender.clone()) {
				panic!("Failed to add client {}: {}", count, error);
			}
		}
	}

	#[test]
	fn should_not_allow_adding_more_clients_than_room_size() {
		let room = Room::new(2);
		for count in 1..=2 {
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = room.add_client_and_return_existing(format!("{}", count), message_sender.clone()) {
				panic!("Failed to add client {}: {}", count, error);
			}
		}

		let message_sender = MessageSender::from(FakeMessageSender::default());
		let result = room.add_client_and_return_existing("elephant".to_string(), message_sender.clone());
		assert!(matches!(result, Err(RoomError::RoomFull)))
	}

	#[test]
	fn should_count_down_clients_once_they_are_removed() {
		// With a room size of one
		let room = Room::new(1);

		// Expect an initial add- and remove work
		let ferris_connection = MessageSender::from(FakeMessageSender::default());
		let (ferris, _) = room
			.add_client_and_return_existing("Ferris".to_string(), ferris_connection)
			.expect("Could not add client!");
		assert!(room.remove_client(ferris.id()), "Could not remove client!");

		// And a subsequent add also works
		let crab_connection = MessageSender::from(FakeMessageSender::default());
		room.add_client_and_return_existing("Crab".to_string(), crab_connection)
			.expect("Could not add client!");
	}

	#[test]
	fn should_eject_the_inserted_medium_once_all_clients_have_left_the_room() {
		let room = Room::new(10);
		let name = "牧瀬 紅莉栖";

		let message_sender = MessageSender::from(FakeMessageSender::default());
		let (makise_kurisu, _) = room
			.add_client_and_return_existing(name.to_string(), message_sender.clone())
			.expect("Failed to add client with same name after first is gone");
		let medium = FixedLengthMedium::new("愛のむきだし".to_string(), Duration::minutes(237));
		room.insert_medium(medium, 0).expect("Failed to insert medium");

		assert!(room.remove_client(makise_kurisu.id()), "Could not remove client!");
		assert_eq!(
			room.medium(),
			VersionedMedium {
				medium: Medium::Empty,
				version: 2
			},
			"A medium was still left in the room!"
		);
	}

	#[test]
	fn should_not_insert_medium_with_smaller_previous_version() {
		let room = Room::new(1);
		room.insert_medium(Medium::Empty, 0).expect("Failed to insert medium"); // increase the version
		assert_eq!(room.medium().version, 1);

		assert!(
			room.insert_medium(Medium::Empty, 0).is_none(),
			"Must not be able to insert"
		);
		assert_eq!(room.medium().version, 1);
	}

	#[test]
	fn should_not_insert_medium_with_larger_previous_version() {
		let room = Room::new(1);
		assert!(
			room.insert_medium(Medium::Empty, 1).is_none(),
			"Must not be able to insert"
		);
		assert_eq!(room.medium().version, 0);
	}

	#[test]
	fn add_client_should_return_empty_list_when_adding_to_an_empty_room() {
		let room = Room::new(10);
		let jake_sender = FakeMessageSender::default();
		let (_, existing_clients) = room
			.add_client_and_return_existing("Jake".to_string(), jake_sender.into())
			.unwrap();
		assert!(existing_clients.is_empty());
	}

	#[test]
	fn add_client_should_return_list_of_existing_clients() {
		let room = Room::new(10);
		let jake_sender = FakeMessageSender::default();
		let (jake, existing_clients) = room
			.add_client_and_return_existing("Jake".to_string(), jake_sender.into())
			.unwrap();
		assert!(existing_clients.is_empty());

		let elwood_sender = FakeMessageSender::default();
		let (_, existing_clients) = room
			.add_client_and_return_existing("Elwood".to_string(), elwood_sender.into())
			.unwrap();
		assert_eq!(existing_clients.len(), 1);
		let existing_jake = &existing_clients[0];
		assert_eq!(jake.id(), existing_jake.id());
		assert_eq!(jake.name(), existing_jake.name());
	}
}
