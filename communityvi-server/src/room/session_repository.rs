use crate::connection::broadcast_buffer::BroadcastBuffer;
use crate::connection::sender::MessageSender;
use crate::room::client::Client;
use crate::room::error::RoomError;
use crate::room::session_id::SessionId;
use crate::room::session_id_sequence::SessionIdSequence;
use crate::user::User;
use std::collections::HashMap;

pub struct SessionRepository {
	maximum_size: usize,
	id_sequence: SessionIdSequence,
	clients_by_id: HashMap<SessionId, Client>,
}

impl SessionRepository {
	pub fn with_limit(limit: usize) -> SessionRepository {
		Self {
			maximum_size: limit,
			id_sequence: Default::default(),
			clients_by_id: Default::default(),
		}
	}

	/// Add a new client, passing in a sender for sending messages to it.
	/// Returns the newly added client.
	pub fn add(&mut self, user: User, message_sender: MessageSender) -> Result<Client, RoomError> {
		if self.clients_by_id.len() >= self.maximum_size {
			return Err(RoomError::RoomFull);
		}

		let id = self.id_sequence.next();
		let broadcast_buffer = BroadcastBuffer::new(self.maximum_size);
		let client = Client::new(id, user, broadcast_buffer, message_sender);

		if self.clients_by_id.insert(id, client.clone()).is_some() {
			unreachable!("There must never be two clients with the same id!");
		}

		Ok(client)
	}

	pub fn remove(&mut self, session_id: SessionId) -> Option<Client> {
		self.clients_by_id.remove(&session_id)
	}

	pub fn is_empty(&self) -> bool {
		self.clients_by_id.is_empty()
	}

	pub fn iter_clients(&self) -> impl Iterator<Item = &Client> {
		self.clients_by_id.values()
	}
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod test {
	use super::*;
	use crate::user::UserRepository;
	use crate::utils::fake_message_sender::FakeMessageSender;

	#[test]
	fn should_track_if_there_are_any_clients_left() {
		let mut user_repository = UserRepository::default();
		let mut session_repository = SessionRepository::with_limit(2);
		let ferris = user_repository.create_user("Ferris").expect("Could not create Ferris!");
		let spidey = user_repository.create_user("Spidey").expect("Could not create Spidey!");

		let ferris_connection = MessageSender::from(FakeMessageSender::default());
		let ferris_client = session_repository
			.add(ferris, ferris_connection)
			.expect("Could not add Ferris!");
		let spidey_connection = MessageSender::from(FakeMessageSender::default());
		let spidey_client = session_repository
			.add(spidey, spidey_connection)
			.expect("Could not add Spidey!");

		session_repository.remove(ferris_client.id());
		assert!(!session_repository.is_empty());

		session_repository.remove(spidey_client.id());
		assert!(session_repository.is_empty());

		// And a subsequent add also works
		let crab = user_repository.create_user("Crab").expect("Could not create Crab!");
		let crab_connection = MessageSender::from(FakeMessageSender::default());
		session_repository
			.add(crab, crab_connection)
			.expect("Could not add client!");
	}

	#[test]
	fn should_allow_adding_clients_up_to_limit() {
		let mut user_repository = UserRepository::default();
		let mut session_repository = SessionRepository::with_limit(2);
		for count in 1..=2 {
			let user = user_repository
				.create_user(&format!("{count}"))
				.expect("Could not create user!");
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = session_repository.add(user, message_sender.clone()) {
				panic!("Failed to add client {count}: {error}");
			}
		}
	}

	#[test]
	fn should_not_allow_adding_more_clients_than_limit() {
		let mut user_repository = UserRepository::default();
		let mut session_repository = SessionRepository::with_limit(2);
		for count in 1..=2 {
			let user = user_repository
				.create_user(&format!("{count}"))
				.expect("Could not create user!");
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = session_repository.add(user, message_sender.clone()) {
				panic!("Failed to add client {count}: {error}");
			}
		}

		let elephant = user_repository.create_user("elephant").expect("Could not create user!");
		let message_sender = MessageSender::from(FakeMessageSender::default());
		let result = session_repository.add(elephant, message_sender);
		assert!(matches!(result, Err(RoomError::RoomFull)));
	}
}
