use super::{SqliteRepository, sqlite_connection};
use crate::chat::model::ChatMessage;
use crate::chat::repository::ChatRepository;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::types::date_time::DateTime;
use crate::types::uuid::Uuid;
use async_trait::async_trait;
use sqlx::query_as;

#[async_trait]
impl ChatRepository for SqliteRepository {
	async fn create(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
		user_name: String,
		message: String,
		created_at: DateTime,
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
