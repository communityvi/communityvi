use crate::chat_message::model::ChatMessage;
use crate::chat_message::repository::ChatMessageRepository;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::sqlite::{SqliteRepository, sqlite_connection};
use async_trait::async_trait;
use sqlx::query_as;
use uuid::Uuid;

#[async_trait]
impl ChatMessageRepository for SqliteRepository {
	async fn get(&self, connection: &mut dyn Connection, uuid: Uuid) -> Result<Option<ChatMessage>, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"
			SELECT
				uuid,
				room_uuid,
				user_uuid,
				user_name,
				message,
				created_at
			 FROM chat_message WHERE uuid = ?1
			",
		)
		.bind(uuid)
		.fetch_optional(connection)
		.await
		.map_err(Into::into)
	}

	async fn create(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
		user_name: &str,
		message: &str,
	) -> Result<ChatMessage, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		let uuid = Uuid::new_v4();
		query_as(
			r"
			INSERT INTO chat_message (
				uuid,
				room_uuid,
				user_uuid,
				user_name,
				message
			) VALUES (?1, ?2, ?3, ?4, ?5)
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
		.fetch_one(connection)
		.await
		.map_err(Into::into)
	}

	async fn remove(&self, connection: &mut dyn Connection, uuid: Uuid) -> Result<(), DatabaseError> {
		todo!()
	}
}
