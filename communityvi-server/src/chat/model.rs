use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow, Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
	pub uuid: Uuid,
	pub room_uuid: Uuid,
	pub user_uuid: Option<Uuid>,
	pub user_name: String,
	pub message: String,
	pub created_at: DateTime<Utc>,
}
