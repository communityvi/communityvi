use crate::user::model::User;
use chrono::{DateTime, Utc};
use sqlx::{Error, FromRow};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
	pub uuid: Uuid,
	pub room_uuid: Uuid,
	pub user: User,
	pub message: String,
	pub created_at: DateTime<Utc>,
}

impl<'row, Row> FromRow<'row, Row> for ChatMessage
where
	Row: sqlx::Row,
	for<'a> &'a str: sqlx::ColumnIndex<Row>,
	Uuid: sqlx::Decode<'row, Row::Database>,
	Uuid: sqlx::Type<Row::Database>,
	String: sqlx::Decode<'row, Row::Database>,
	String: sqlx::Type<Row::Database>,
	DateTime<Utc>: sqlx::Decode<'row, Row::Database>,
	DateTime<Utc>: sqlx::Type<Row::Database>,
{
	fn from_row(row: &'row Row) -> Result<Self, Error> {
		Ok(Self {
			uuid: row.try_get("uuid")?,
			room_uuid: row.try_get("room_uuid")?,
			user: User {
				uuid: row.try_get("user_uuid")?,
				name: row.try_get("user_name")?,
			},
			message: row.try_get("message")?,
			created_at: row.try_get("created_at")?,
		})
	}
}
