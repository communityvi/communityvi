use crate::connection::client::ClientConnection;
use crate::message::ServerResponse;
use crate::room::client::Client;
use crate::room::client_id::ClientId;
use crate::room::client_id_sequence::ClientIdSequence;
use crate::room::error::RoomError;
use crate::room::state::State;
use dashmap::{DashMap, DashSet};
use futures::FutureExt;
use log::info;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::time::Duration;
use unicode_skeleton::UnicodeSkeleton;

pub mod client;
pub mod client_id;
mod client_id_sequence;
pub mod error;
mod state;

#[derive(Clone)]
pub struct Room {
	inner: Arc<Inner>,
}

struct Inner {
	client_id_sequence: ClientIdSequence,
	client_names: DashSet<String>,
	clients: DashMap<ClientId, Client>,
	client_count: AtomicUsize,
	state: State,
	room_size_limit: usize,
}

impl Room {
	pub fn new(room_size_limit: usize) -> Self {
		let inner = Inner {
			client_id_sequence: Default::default(),
			client_names: Default::default(),
			clients: Default::default(),
			client_count: AtomicUsize::new(0),
			state: Default::default(),
			room_size_limit,
		};
		Self { inner: Arc::new(inner) }
	}

	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub fn add_client(&self, name: String, connection: ClientConnection) -> Result<Client, RoomError> {
		if name.trim().is_empty() {
			return Err(RoomError::EmptyClientName);
		}

		const MAX_NAME_LENGTH: usize = 256;
		if name.len() > MAX_NAME_LENGTH {
			return Err(RoomError::ClientNameTooLong);
		}

		if !self.try_incrementing_client_count() {
			return Err(RoomError::RoomFull);
		}

		if !self.inner.client_names.insert(normalized_name(name.as_str())) {
			self.inner.client_count.fetch_sub(1, SeqCst);
			return Err(RoomError::ClientNameAlreadyInUse);
		}

		let client_id = self.inner.client_id_sequence.next();
		let client = Client::new(client_id, name, connection, self.clone());

		if self.inner.clients.insert(client_id, client.clone()).is_some() {
			unreachable!("There must never be two clients with the same id!")
		}

		Ok(client)
	}

	// Does a compare and swap until the room count has been incremented (true) or is `room_size_limit` (false).
	fn try_incrementing_client_count(&self) -> bool {
		let mut current_count = self.inner.client_count.load(SeqCst);
		loop {
			if current_count == self.inner.room_size_limit {
				return false;
			}

			match self
				.inner
				.client_count
				.compare_exchange(current_count, current_count + 1, SeqCst, SeqCst)
			{
				Ok(_) => return true,
				Err(previous_count) => current_count = previous_count,
			}
		}
	}

	pub fn remove_client(&self, client_id: ClientId) -> bool {
		self.inner
			.clients
			.remove(&client_id)
			.map(|(_, client)| self.inner.client_names.remove(&normalized_name(client.name())))
			.is_some()
	}

	pub async fn broadcast(&self, response: ServerResponse) {
		let futures: Vec<_> = self
			.inner
			.clients
			.iter()
			.map(|entry| entry.value().clone())
			.map(move |client| {
				let response = response.clone();
				async move {
					if !client.send(response).await {
						info!("Client with id {} has gone away during broadcast.", client.id());
					}
				}
			})
			.collect();
		futures::future::join_all(futures).map(|_: Vec<()>| ()).await
	}

	pub fn current_reference_time(&self) -> Duration {
		self.inner.state.current_reference_time()
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
	use crate::utils::fake_connection::FakeClientConnection;

	#[test]
	fn should_not_add_client_with_empty_name() {
		let room = Room::new(10);
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		let result = room.add_client("".to_string(), client_connection.clone());

		assert!(matches!(result, Err(RoomError::EmptyClientName)));
	}

	#[test]
	fn should_not_add_client_with_blank_name() {
		let room = Room::new(10);
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		let result = room.add_client("  	 ".to_string(), client_connection.clone());

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
		assert_eq!(normalized_name("аррӏе.com"), "apple.com");
	}

	#[test]
	fn should_not_add_two_clients_with_the_same_name() {
		let room = Room::new(10);
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		room.add_client("Anorak  ".to_string(), client_connection.clone())
			.expect("First add did not succeed!");
		let result = room.add_client("   Anorak".to_string(), client_connection.clone());

		assert!(matches!(result, Err(RoomError::ClientNameAlreadyInUse)));
	}

	#[test]
	fn should_allow_adding_client_with_the_same_name_after_first_has_been_removed() {
		let room = Room::new(10);
		let name = "牧瀬 紅莉栖";

		{
			let client_connection = ClientConnection::from(FakeClientConnection::default());
			let client = room
				.add_client(name.to_string(), client_connection.clone())
				.expect("Failed to add client");
			room.remove_client(client.id());
		}

		let client_connection = ClientConnection::from(FakeClientConnection::default());
		room.add_client(name.to_string(), client_connection.clone())
			.expect("Failed to add client with same name after first is gone");
	}

	#[test]
	fn should_allow_adding_client_with_name_no_longer_than_256_bytes() {
		let long_name = String::from_utf8(vec![0x41u8; 256]).unwrap();
		let room = Room::new(10);
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		room.add_client(long_name.to_string(), client_connection.clone())
			.expect("Failed to add client with name that is not too long");
	}

	#[test]
	fn should_not_allow_adding_client_with_name_longer_than_256_bytes() {
		let long_name = String::from_utf8(vec![0x41u8; 257]).unwrap();
		let room = Room::new(10);
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		let result = room.add_client(long_name.to_string(), client_connection.clone());

		assert!(matches!(result, Err(RoomError::ClientNameTooLong)));
	}

	#[test]
	fn should_allow_adding_clients_up_to_room_size_limit() {
		let room = Room::new(2);
		for count in 1..=2 {
			let client_connection = ClientConnection::from(FakeClientConnection::default());

			if let Err(error) = room.add_client(format!("{}", count), client_connection.clone()) {
				panic!("Failed to add client {}: {}", count, error);
			}
		}
	}

	#[test]
	fn should_not_allow_adding_more_clients_than_room_size() {
		let room = Room::new(2);
		for count in 1..=2 {
			let client_connection = ClientConnection::from(FakeClientConnection::default());

			if let Err(error) = room.add_client(format!("{}", count), client_connection.clone()) {
				panic!("Failed to add client {}: {}", count, error);
			}
		}

		let client_connection = ClientConnection::from(FakeClientConnection::default());
		let result = room.add_client("elephant".to_string(), client_connection.clone());
		assert!(matches!(result, Err(RoomError::RoomFull)))
	}
}
