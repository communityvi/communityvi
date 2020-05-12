use crate::connection::sender::MessageSender;
use crate::room::client::Client;
use crate::room::client_id::ClientId;
use crate::room::client_id_sequence::ClientIdSequence;
use crate::room::error::RoomError;
use std::collections::{HashMap, HashSet};
use unicode_skeleton::UnicodeSkeleton;

pub struct Clients {
	client_id_sequence: ClientIdSequence,
	names: HashSet<String>,
	clients_by_id: HashMap<ClientId, Client>,
	maximum_size: usize,
}

impl Clients {
	pub fn with_limit(limit: usize) -> Clients {
		Self {
			client_id_sequence: Default::default(),
			names: Default::default(),
			clients_by_id: Default::default(),
			maximum_size: limit,
		}
	}

	/// Add a new client, passing in a sender for sending messages to it.
	/// Returns the newly added client and a list of clients that had existed prior to adding this one.
	pub fn add_and_return_existing(
		&mut self,
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

		if self.clients_by_id.len() >= self.maximum_size {
			return Err(RoomError::RoomFull);
		}

		if !self.names.insert(normalized_name(name.as_str())) {
			return Err(RoomError::ClientNameAlreadyInUse);
		}

		let client_id = self.client_id_sequence.next();
		let client = Client::new(client_id, name, message_sender);

		let existing_clients = self.clients_by_id.iter().map(|(_id, client)| client.clone()).collect();
		if self.clients_by_id.insert(client_id, client.clone()).is_some() {
			unreachable!("There must never be two clients with the same id!")
		}

		Ok((client, existing_clients))
	}

	pub fn remove(&mut self, client_id: ClientId) -> usize {
		self.clients_by_id
			.remove(&client_id)
			.map(|client| self.names.remove(&normalized_name(client.name())));
		self.clients_by_id.len()
	}

	pub fn iter(&self) -> impl Iterator<Item = (ClientId, &Client)> {
		self.clients_by_id
			.iter()
			.map(|(&client_id, client)| (client_id, client))
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
	use crate::utils::fake_message_sender::FakeMessageSender;

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
	fn should_not_add_with_empty_name() {
		let mut clients = Clients::with_limit(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		let result = clients.add_and_return_existing("".to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::EmptyClientName)));
	}

	#[test]
	fn should_not_add_with_blank_name() {
		let mut clients = Clients::with_limit(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		let result = clients.add_and_return_existing("  	 ".to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::EmptyClientName)));
	}

	#[test]
	fn should_not_add_two_clients_with_the_same_name() {
		let mut clients = Clients::with_limit(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		clients
			.add_and_return_existing("Anorak  ".to_string(), message_sender.clone())
			.expect("First add did not succeed!");
		let result = clients.add_and_return_existing("   Anorak".to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::ClientNameAlreadyInUse)));
	}

	#[test]
	fn should_allow_adding_client_with_the_same_name_after_first_has_been_removed() {
		let mut clients = Clients::with_limit(10);
		let name = "牧瀬 紅莉栖";

		{
			let message_sender = MessageSender::from(FakeMessageSender::default());
			let (client, _) = clients
				.add_and_return_existing(name.to_string(), message_sender.clone())
				.expect("Failed to add client");
			clients.remove(client.id());
		}

		let message_sender = MessageSender::from(FakeMessageSender::default());
		clients
			.add_and_return_existing(name.to_string(), message_sender.clone())
			.expect("Failed to add client with same name after first is gone");
	}

	#[test]
	fn should_allow_adding_client_with_name_no_longer_than_256_bytes() {
		let long_name = String::from_utf8(vec![0x41u8; 256]).unwrap();
		let mut clients = Clients::with_limit(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		clients
			.add_and_return_existing(long_name.to_string(), message_sender.clone())
			.expect("Failed to add client with name that is not too long");
	}

	#[test]
	fn should_not_allow_adding_client_with_name_longer_than_256_bytes() {
		let long_name = String::from_utf8(vec![0x41u8; 257]).unwrap();
		let mut clients = Clients::with_limit(10);
		let message_sender = MessageSender::from(FakeMessageSender::default());

		let result = clients.add_and_return_existing(long_name.to_string(), message_sender.clone());

		assert!(matches!(result, Err(RoomError::ClientNameTooLong)));
	}

	#[test]
	fn add_should_return_empty_list_when_adding_to_an_empty_list() {
		let mut clients = Clients::with_limit(10);
		let jake_sender = FakeMessageSender::default();
		let (_, existing_clients) = clients
			.add_and_return_existing("Jake".to_string(), jake_sender.into())
			.unwrap();
		assert!(existing_clients.is_empty());
	}

	#[test]
	fn add_should_return_list_of_existing_clients() {
		let mut clients = Clients::with_limit(10);
		let jake_sender = FakeMessageSender::default();
		let (jake, existing_clients) = clients
			.add_and_return_existing("Jake".to_string(), jake_sender.into())
			.unwrap();
		assert!(existing_clients.is_empty());

		let elwood_sender = FakeMessageSender::default();
		let (_, existing_clients) = clients
			.add_and_return_existing("Elwood".to_string(), elwood_sender.into())
			.unwrap();
		assert_eq!(existing_clients.len(), 1);
		let existing_jake = &existing_clients[0];
		assert_eq!(jake.id(), existing_jake.id());
		assert_eq!(jake.name(), existing_jake.name());
	}

	#[test]
	fn should_count_down_clients_once_they_are_removed() {
		let mut clients = Clients::with_limit(2);

		let ferris_connection = MessageSender::from(FakeMessageSender::default());
		let (ferris, _) = clients
			.add_and_return_existing("Ferris".to_string(), ferris_connection)
			.expect("Could not add Ferris!");
		let spidey_connection = MessageSender::from(FakeMessageSender::default());
		let (spidey, _) = clients
			.add_and_return_existing("Spidey".to_string(), spidey_connection)
			.expect("Could not add Spidey!");
		assert_eq!(clients.remove(ferris.id()), 1);
		assert_eq!(clients.remove(spidey.id()), 0);

		// And a subsequent add also works
		let crab_connection = MessageSender::from(FakeMessageSender::default());
		clients
			.add_and_return_existing("Crab".to_string(), crab_connection)
			.expect("Could not add client!");
	}

	#[test]
	fn should_allow_adding_clients_up_to_limit() {
		let mut clients = Clients::with_limit(2);
		for count in 1..=2 {
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = clients.add_and_return_existing(format!("{}", count), message_sender.clone()) {
				panic!("Failed to add client {}: {}", count, error);
			}
		}
	}

	#[test]
	fn should_not_allow_adding_more_clients_than_limit() {
		let mut clients = Clients::with_limit(2);
		for count in 1..=2 {
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = clients.add_and_return_existing(format!("{}", count), message_sender.clone()) {
				panic!("Failed to add client {}: {}", count, error);
			}
		}

		let message_sender = MessageSender::from(FakeMessageSender::default());
		let result = clients.add_and_return_existing("elephant".to_string(), message_sender.clone());
		assert!(matches!(result, Err(RoomError::RoomFull)))
	}
}
