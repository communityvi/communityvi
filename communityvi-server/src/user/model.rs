use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow, Clone, Debug, PartialEq, Eq)]
pub struct User {
	pub uuid: Uuid,
	pub name: String,
	pub normalized_name: String,
}
