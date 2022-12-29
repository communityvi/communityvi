use crate::connection::broadcast_buffer::BroadcastBuffer;
use crate::connection::sender::MessageSender;
use crate::room::client::Client;
use crate::room::client_id::ClientId;
use crate::room::client_id_sequence::ClientIdSequence;
use crate::room::error::RoomError;
use crate::user::UserRepository;
use std::collections::HashMap;

pub struct Clients {
	user_repository: UserRepository,
	client_id_sequence: ClientIdSequence,
	clients_by_id: HashMap<ClientId, Client>,
	maximum_size: usize,
}

impl Clients {
	pub fn with_limit(limit: usize) -> Clients {
		Self {
			user_repository: Default::default(),
			client_id_sequence: Default::default(),
			clients_by_id: Default::default(),
			maximum_size: limit,
		}
	}

	/// Add a new client, passing in a sender for sending messages to it.
	/// Returns the newly added client and a list of clients that had existed prior to adding this one.
	pub fn add_and_return_existing(
		&mut self,
		name: &str,
		message_sender: MessageSender,
	) -> Result<(Client, Vec<Client>), RoomError> {
		if self.clients_by_id.len() >= self.maximum_size {
			return Err(RoomError::RoomFull);
		}

		let user = self.user_repository.create_user(name)?;

		let client_id = self.client_id_sequence.next();
		let broadcast_buffer = BroadcastBuffer::new(self.maximum_size);
		let client = Client::new(client_id, user, broadcast_buffer, message_sender);

		let existing_clients = self.clients_by_id.values().cloned().collect();
		if self.clients_by_id.insert(client_id, client.clone()).is_some() {
			unreachable!("There must never be two clients with the same id!");
		}

		Ok((client, existing_clients))
	}

	pub fn remove(&mut self, client_id: ClientId) -> usize {
		if let Some(client) = self.clients_by_id.remove(&client_id) {
			self.user_repository.remove(client.user());
		}
		self.clients_by_id.len()
	}

	pub fn iter_clients(&self) -> impl Iterator<Item = &Client> {
		self.clients_by_id.values()
	}
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod test {
	use super::*;
	use crate::utils::fake_message_sender::FakeMessageSender;

	#[test]
	fn add_should_return_empty_list_when_adding_to_an_empty_list() {
		let mut clients = Clients::with_limit(10);
		let jake_sender = FakeMessageSender::default();
		let (_, existing_clients) = clients.add_and_return_existing("Jake", jake_sender.into()).unwrap();
		assert!(existing_clients.is_empty());
	}

	#[test]
	fn add_should_return_list_of_existing_clients() {
		let mut clients = Clients::with_limit(10);
		let jake_sender = FakeMessageSender::default();
		let (jake, existing_clients) = clients.add_and_return_existing("Jake", jake_sender.into()).unwrap();
		assert!(existing_clients.is_empty());

		let elwood_sender = FakeMessageSender::default();
		let (_, existing_clients) = clients.add_and_return_existing("Elwood", elwood_sender.into()).unwrap();
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
			.add_and_return_existing("Ferris", ferris_connection)
			.expect("Could not add Ferris!");
		let spidey_connection = MessageSender::from(FakeMessageSender::default());
		let (spidey, _) = clients
			.add_and_return_existing("Spidey", spidey_connection)
			.expect("Could not add Spidey!");
		assert_eq!(clients.remove(ferris.id()), 1);
		assert_eq!(clients.remove(spidey.id()), 0);

		// And a subsequent add also works
		let crab_connection = MessageSender::from(FakeMessageSender::default());
		clients
			.add_and_return_existing("Crab", crab_connection)
			.expect("Could not add client!");
	}

	#[test]
	fn should_allow_adding_clients_up_to_limit() {
		let mut clients = Clients::with_limit(2);
		for count in 1..=2 {
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = clients.add_and_return_existing(&format!("{count}"), message_sender.clone()) {
				panic!("Failed to add client {count}: {error}");
			}
		}
	}

	#[test]
	fn should_not_allow_adding_more_clients_than_limit() {
		let mut clients = Clients::with_limit(2);
		for count in 1..=2 {
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = clients.add_and_return_existing(&format!("{count}"), message_sender.clone()) {
				panic!("Failed to add client {count}: {error}");
			}
		}

		let message_sender = MessageSender::from(FakeMessageSender::default());
		let result = clients.add_and_return_existing("elephant", message_sender);
		assert!(matches!(result, Err(RoomError::RoomFull)));
	}
}
