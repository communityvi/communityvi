use crate::chat::model::ChatMessage;
use crate::chat::repository::ChatRepository;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::libsql::{LibSqlRepository, libsql_connection};
use crate::types::date_time::DateTime;
use crate::types::uuid::Uuid;
use anyhow::anyhow;
use async_trait::async_trait;

#[async_trait]
impl ChatRepository for LibSqlRepository {
	async fn create(
		&self,
		connection: &dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
		user_name: String,
		message: String,
		created_at: DateTime,
	) -> Result<ChatMessage, DatabaseError> {
		let connection = libsql_connection(connection)?;

		let uuid = Uuid::new_v4();
		let mut rows = connection
			.query(
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
				(uuid, room_uuid, user_uuid, user_name, message, created_at),
			)
			.await?;

		rows.next()
			.await?
			.ok_or_else(|| DatabaseError::NotFound(anyhow!("not found")))?
			.try_into()
			.map_err(DatabaseError::Decode)
	}
}
