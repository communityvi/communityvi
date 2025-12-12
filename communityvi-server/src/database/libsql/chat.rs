use crate::chat::model::ChatMessage;
use crate::chat::repository::ChatRepository;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::libsql::LibSqlRepository;
use crate::types::uuid::Uuid;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[async_trait]
impl ChatRepository for LibSqlRepository {
	async fn create(
		&self,
		_connection: &mut dyn Connection,
		_room_uuid: Uuid,
		_user_uuid: Uuid,
		_user_name: String,
		_message: String,
		_created_at: DateTime<Utc>,
	) -> Result<ChatMessage, DatabaseError> {
		unimplemented!()
	}
}
