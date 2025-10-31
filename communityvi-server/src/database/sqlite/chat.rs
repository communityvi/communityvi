use super::{SqliteRepository, sqlite_connection};
use crate::chat::model::ChatMessage;
use crate::chat::repository::ChatRepository;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::query_as;
use uuid::Uuid;

#[async_trait]
impl ChatRepository for SqliteRepository {
	async fn create(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
		user_name: String,
		message: String,
		created_at: DateTime<Utc>,
	) -> Result<ChatMessage, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		let uuid = Uuid::new_v4();
		query_as(
			"INSERT INTO chat_message(
				uuid, room_uuid, user_uuid, user_name, message, created_at
			) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
			RETURNING
				uuid,
				room_uuid,
				user_uuid,
				user_name,
				message,
				created_at
			",
		)
		.bind(uuid)
		.bind(room_uuid)
		.bind(user_uuid)
		.bind(user_name)
		.bind(message)
		.bind(created_at)
		.fetch_one(connection)
		.await
		.map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::sqlite::test_utils::connection;
	use crate::database::{Connection, Repository};
	use crate::room::model::Room;
	use crate::user::model::User;

	#[tokio::test]
	async fn creates_chat_message() {
		let mut connection = connection().await;
		let user = user(&mut *connection, "alice").await;
		let room = room(&mut *connection, "lobby").await;
		let message = "Hello world!".to_string();
		let created_at = Utc::now();

		let ChatMessage {
			uuid,
			room_uuid,
			user: chat_user,
			message: chat_message,
			created_at: chat_created_at,
		} = SqliteRepository
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
		assert_eq!(user.uuid, chat_user.uuid);
		assert_eq!(user.name, chat_user.name);
		assert_eq!(message, chat_message);
		// Compare timestamps at second precision to avoid driver precision differences
		assert_eq!(created_at.timestamp(), chat_created_at.timestamp());
	}

	#[tokio::test]
	async fn rejects_empty_message() {
		let mut connection = connection().await;
		let user = user(&mut *connection, "bob").await;
		let room = room(&mut *connection, "general").await;

		let result = SqliteRepository
			.chat()
			.create(
				&mut *connection,
				room.uuid,
				user.uuid,
				user.name.clone(),
				String::new(), // empty message should violate check constraint
				Utc::now(),
			)
			.await;

		match result {
			Err(DatabaseError::OtherConstraintViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected constraint violation when creating empty message"),
			Err(err) => panic!("Unexpected error: {err:?}"),
		}
	}

	async fn user(connection: &mut dyn Connection, name: &str) -> User {
		SqliteRepository
			.user()
			.create(connection, name)
			.await
			.expect("Failed to create user")
	}

	async fn room(connection: &mut dyn Connection, name: &str) -> Room {
		SqliteRepository
			.room()
			.create(connection, name)
			.await
			.expect("Failed to create room")
	}
}
