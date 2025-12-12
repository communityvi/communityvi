use crate::types::uuid::Uuid;
use sqlx::FromRow;

#[derive(FromRow, Clone, Debug, PartialEq, Eq)]
pub struct User {
	pub uuid: Uuid,
	pub name: String,
	pub normalized_name: String,
}
