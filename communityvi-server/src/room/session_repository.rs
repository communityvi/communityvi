use crate::connection::broadcast_buffer::BroadcastBuffer;
use crate::connection::sender::MessageSender;
use crate::room::client::Client;
use crate::room::error::RoomError;
use crate::room::session_id::SessionId;
use crate::room::session_id_sequence::SessionIdSequence;
use crate::user::model::User;
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
	/// Returns the newly added client and a list of clients that had existed prior to adding this one.
	pub fn add_and_return_existing(
		&mut self,
		user: User,
		message_sender: MessageSender,
	) -> Result<(Client, Vec<Client>), RoomError> {
		if self.clients_by_id.len() >= self.maximum_size {
			return Err(RoomError::RoomFull);
		}

		let id = self.id_sequence.next();
		let broadcast_buffer = BroadcastBuffer::new(self.maximum_size);
		let client = Client::new(id, user, broadcast_buffer, message_sender);

		let existing_clients = self.clients_by_id.values().cloned().collect();
		if self.clients_by_id.insert(id, client.clone()).is_some() {
			unreachable!("There must never be two clients with the same id!");
		}

		Ok((client, existing_clients))
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
	use crate::database::test::{DefaultTestFactory, TestFactory};
	use crate::user::UserService;
	use crate::utils::fake_message_sender::FakeMessageSender;

	#[tokio::test]
	async fn add_should_return_empty_list_when_adding_to_an_empty_list() {
		let user_repository = user_repository();
		let mut connection = DefaultTestFactory::connection().await;
		let mut session_repository = SessionRepository::with_limit(10);
		let jake = user_repository
			.create_user("Jake", connection.as_mut())
			.await
			.expect("Could not create user");
		let jake_sender = FakeMessageSender::default();
		let (_, existing_clients) = session_repository
			.add_and_return_existing(jake, jake_sender.into())
			.unwrap();
		assert!(existing_clients.is_empty());
	}

	#[tokio::test]
	async fn add_should_return_list_of_existing_clients() {
		let user_repository = user_repository();
		let mut connection = DefaultTestFactory::connection().await;
		let mut session_repository = SessionRepository::with_limit(10);
		let jake = user_repository
			.create_user("Jake", connection.as_mut())
			.await
			.expect("Could not create user");
		let elwood = user_repository
			.create_user("Elwood", connection.as_mut())
			.await
			.expect("Could not create user");
		let jake_sender = FakeMessageSender::default();
		let (jake, existing_clients) = session_repository
			.add_and_return_existing(jake, jake_sender.into())
			.unwrap();
		assert!(existing_clients.is_empty());

		let elwood_sender = FakeMessageSender::default();
		let (_, existing_clients) = session_repository
			.add_and_return_existing(elwood, elwood_sender.into())
			.unwrap();
		assert_eq!(existing_clients.len(), 1);
		let existing_jake = &existing_clients[0];
		assert_eq!(jake.id(), existing_jake.id());
		assert_eq!(jake.name(), existing_jake.name());
	}

	#[tokio::test]
	async fn should_track_if_there_are_any_clients_left() {
		let user_repository = user_repository();
		let mut connection = DefaultTestFactory::connection().await;
		let mut session_repository = SessionRepository::with_limit(2);
		let ferris = user_repository
			.create_user("Ferris", connection.as_mut())
			.await
			.expect("Could not create Ferris!");
		let spidey = user_repository
			.create_user("Spidey", connection.as_mut())
			.await
			.expect("Could not create Spidey!");

		let ferris_connection = MessageSender::from(FakeMessageSender::default());
		let (ferris_client, _) = session_repository
			.add_and_return_existing(ferris, ferris_connection)
			.expect("Could not add Ferris!");
		let spidey_connection = MessageSender::from(FakeMessageSender::default());
		let (spidey_client, _) = session_repository
			.add_and_return_existing(spidey, spidey_connection)
			.expect("Could not add Spidey!");

		session_repository.remove(ferris_client.id());
		assert!(!session_repository.is_empty());

		session_repository.remove(spidey_client.id());
		assert!(session_repository.is_empty());

		// And a subsequent add also works
		let crab = user_repository
			.create_user("Crab", connection.as_mut())
			.await
			.expect("Could not create Crab!");
		let crab_connection = MessageSender::from(FakeMessageSender::default());
		session_repository
			.add_and_return_existing(crab, crab_connection)
			.expect("Could not add client!");
	}

	#[tokio::test]
	async fn should_allow_adding_clients_up_to_limit() {
		let user_repository = user_repository();
		let mut connection = DefaultTestFactory::connection().await;
		let mut session_repository = SessionRepository::with_limit(2);
		for count in 1..=2 {
			let user = user_repository
				.create_user(&format!("{count}"), connection.as_mut())
				.await
				.expect("Could not create user!");
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = session_repository.add_and_return_existing(user, message_sender.clone()) {
				panic!("Failed to add client {count}: {error}");
			}
		}
	}

	#[tokio::test]
	async fn should_not_allow_adding_more_clients_than_limit() {
		let user_repository = user_repository();
		let mut connection = DefaultTestFactory::connection().await;
		let mut session_repository = SessionRepository::with_limit(2);
		for count in 1..=2 {
			let user = user_repository
				.create_user(&format!("{count}"), connection.as_mut())
				.await
				.expect("Could not create user!");
			let message_sender = MessageSender::from(FakeMessageSender::default());

			if let Err(error) = session_repository.add_and_return_existing(user, message_sender.clone()) {
				panic!("Failed to add client {count}: {error}");
			}
		}

		let elephant = user_repository
			.create_user("elephant", connection.as_mut())
			.await
			.expect("Could not create user!");
		let message_sender = MessageSender::from(FakeMessageSender::default());
		let result = session_repository.add_and_return_existing(elephant, message_sender);
		assert!(matches!(result, Err(RoomError::RoomFull)));
	}

	fn user_repository() -> UserService {
		let repository = DefaultTestFactory::repository();
		UserService::new(repository)
	}
}
