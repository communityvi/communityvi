use crate::chat::model;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[async_trait]
pub trait ChatRepository: Send + Sync + 'static {
	async fn create(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
		user_name: String,
		message: String,
		created_at: DateTime<Utc>,
	) -> Result<model::ChatMessage, DatabaseError>;
}
