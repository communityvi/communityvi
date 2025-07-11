use sqlx::Decode;
use uuid::Uuid;

#[derive(Decode)]
pub struct Room {
	pub uuid: Uuid,
	pub name: String,
	pub medium_uuid: Option<Uuid>,
}
