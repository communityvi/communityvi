use crate::chat_message::model::ChatMessage;
use crate::database::Connection;
use crate::database::error::DatabaseError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ChatMessageRepository {
	async fn get(&self, connection: &mut dyn Connection, uuid: Uuid) -> Result<Option<ChatMessage>, DatabaseError>;
	async fn create(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
		user_name: &str,
		message: &str,
	) -> Result<ChatMessage, DatabaseError>;
	async fn remove(&self, connection: &mut dyn Connection, uuid: Uuid) -> Result<(), DatabaseError>;
}
