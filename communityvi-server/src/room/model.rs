use crate::types::uuid::Uuid;
use sqlx::FromRow;

#[derive(FromRow, Clone, Debug, PartialEq, Eq)]
pub struct Room {
	pub uuid: Uuid,
	pub name: String,
	pub medium_uuid: Option<Uuid>,
}
