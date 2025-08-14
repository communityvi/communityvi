use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow, Debug, PartialEq, Eq)]
pub struct Room {
	pub uuid: Uuid,
	pub name: String,
	pub medium_uuid: Option<Uuid>,
}
