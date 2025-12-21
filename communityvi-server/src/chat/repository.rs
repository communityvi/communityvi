use crate::chat::model;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::types::date_time::DateTime;
use crate::types::uuid::Uuid;
use async_trait::async_trait;

#[cfg(test)]
mod tests;

#[async_trait]
pub trait ChatRepository: Send + Sync + 'static {
	async fn create(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
		user_name: String,
		message: String,
		created_at: DateTime,
	) -> Result<model::ChatMessage, DatabaseError>;
}
