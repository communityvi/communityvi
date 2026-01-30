#[generic_tests::define(attrs(tokio::test))]
mod chat_tests {
	use crate::chat::model::ChatMessage;
	use crate::database::error::DatabaseError;
	use crate::database::libsql::test_utils::LibSqlTestFactory;
	use crate::database::test::TestFactory;
	use crate::database::{Connection, Repository};
	use crate::room::model::Room;
	use crate::user::model::User;
	use crate::user::normalize_name;
	use chrono::Utc;

	#[tokio::test]
	async fn creates_chat_message<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let user = user(repository.as_ref(), &mut *connection, "alice").await;
		let room = room(repository.as_ref(), &mut *connection, "lobby").await;
		let message = "Hello world!".to_string();
		let created_at = Utc::now().into();

		let ChatMessage {
			uuid,
			room_uuid,
			user_uuid,
			user_name,
			message: chat_message,
			created_at: chat_created_at,
		} = repository
			.chat()
			.create(
				&mut *connection,
				room.uuid,
				user.uuid,
				user.name.clone(),
				message.clone(),
				created_at,
			)
			.await
			.expect("Failed to create chat message");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!(room.uuid, room_uuid);
		assert_eq!(Some(user.uuid), user_uuid);
		assert_eq!(user.name, user_name);
		assert_eq!(message, chat_message);
		// Compare timestamps at second precision to avoid driver precision differences
		assert_eq!(created_at.timestamp(), chat_created_at.timestamp());
	}

	#[tokio::test]
	async fn rejects_empty_message<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let user = user(repository.as_ref(), &mut *connection, "bob").await;
		let room = room(repository.as_ref(), &mut *connection, "general").await;

		let result = repository
			.chat()
			.create(
				&mut *connection,
				room.uuid,
				user.uuid,
				user.name.clone(),
				String::new(), // empty message should violate check constraint
				Utc::now().into(),
			)
			.await;

		match result {
			Err(DatabaseError::OtherConstraintViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected constraint violation when creating empty message"),
			Err(err) => panic!("Unexpected error: {err:?}"),
		}
	}

	async fn user(repository: &dyn Repository, connection: &mut dyn Connection, name: &str) -> User {
		repository
			.user()
			.create(connection, name, &normalize_name(name))
			.await
			.expect("Failed to create user")
	}

	async fn room(repository: &dyn Repository, connection: &mut dyn Connection, name: &str) -> Room {
		repository
			.room()
			.create(connection, name)
			.await
			.expect("Failed to create room")
	}

	#[instantiate_tests(<LibSqlTestFactory>)]
	mod libsql {}
}
