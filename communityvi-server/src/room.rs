use crate::connection::client::ClientConnection;
use crate::message::ServerResponse;
use crate::room::client::{Client, ClientId};
use crate::room::client_handle::ClientHandle;
use crate::room::client_id_sequence::ClientIdSequence;
use crate::room::error::RoomError;
use crate::room::state::State;
use dashmap::{DashMap, DashSet};
use futures::FutureExt;
use log::info;
use std::sync::Arc;
use std::time::Duration;
use unicode_skeleton::UnicodeSkeleton;

pub mod client;
pub mod client_handle;
mod client_id_sequence;
pub mod error;
mod state;

pub(self) type ClientReference<'a> = dashmap::mapref::one::Ref<'a, ClientId, Client>;

#[derive(Default)]
pub struct Room {
	client_id_sequence: ClientIdSequence,
	client_names: DashSet<String>,
	clients: DashMap<ClientId, Client>,
	state: State,
}

impl Room {
	/// Add a new client to the room, passing in a sender for sending messages to it. Returns it's id
	pub fn add_client(self: &Arc<Room>, name: String, connection: ClientConnection) -> Result<ClientHandle, RoomError> {
		if name.trim().is_empty() {
			return Err(RoomError::EmptyClientName);
		}

		if !self.client_names.insert(normalized_name(name.as_str())) {
			return Err(RoomError::ClientNameAlreadyInUse);
		}

		let client_id = self.client_id_sequence.next();
		let client = Client { name, connection };

		if self.clients.insert(client_id, client).is_some() {
			unreachable!("There must never be two clients with the same id!")
		}

		Ok(ClientHandle::new(client_id, self.clone()))
	}

	/// Remove a client.
	/// IMPORTANT: This is only to be used by `ClientHandle` when being dropped.
	pub(self) fn remove_client(&self, client_id: ClientId) -> bool {
		self.clients.remove(&client_id).is_some()
	}

	/// Look up a client by it's ID.
	/// IMPORTANT: This is only to be used by `ClientHandle` and `MaybeClientHandle`.
	pub(self) fn client_reference_by_id(&self, client_id: ClientId) -> Option<ClientReference> {
		self.clients.get(&client_id).map(ClientReference::from)
	}

	pub async fn broadcast(&self, response: ServerResponse) {
		let futures: Vec<_> = self
			.clients
			.iter()
			.map(|entry| (*entry.key(), entry.connection.clone()))
			.map(move |(id, connection)| {
				let response = response.clone();
				async move {
					if connection.send(response).await.is_err() {
						info!("Client with id {} has gone away during broadcast.", id);
					}
				}
			})
			.collect();
		futures::future::join_all(futures).map(|_: Vec<()>| ()).await
	}

	pub fn current_reference_time(&self) -> Duration {
		self.state.current_reference_time()
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
		let room = Arc::new(Room::default());
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		let result = room.add_client("".to_string(), client_connection.clone());

		matches!(result, Err(RoomError::EmptyClientName));
	}

	#[test]
	fn should_not_add_client_with_blank_name() {
		let room = Arc::new(Room::default());
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		let result = room.add_client("  	 ".to_string(), client_connection.clone());

		matches!(result, Err(RoomError::EmptyClientName));
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
		let room = Arc::new(Room::default());
		let client_connection = ClientConnection::from(FakeClientConnection::default());

		room.add_client("Anorak  ".to_string(), client_connection.clone())
			.expect("First add did not succeed!");
		let result = room.add_client("   Anorak".to_string(), client_connection.clone());

		matches!(result, Err(RoomError::ClientNameAlreadyInUse));
	}
}
