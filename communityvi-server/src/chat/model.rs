use crate::types::date_time::DateTime;
use crate::types::uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
	pub uuid: Uuid,
	pub room_uuid: Uuid,
	pub user_uuid: Option<Uuid>,
	pub user_name: String,
	pub message: String,
	pub created_at: DateTime,
}

impl TryFrom<libsql::Row> for ChatMessage {
	type Error = anyhow::Error;

	fn try_from(row: libsql::Row) -> Result<Self, Self::Error> {
		let uuid = row.get_value(0)?;
		let room_uuid = row.get_value(1)?;
		let user_uuid = row.get_value(2)?;
		let user_name = row.get(3)?;
		let message = row.get(4)?;
		let created_at = row.get_value(5)?;

		Ok(Self {
			uuid: uuid.try_into()?,
			room_uuid: room_uuid.try_into()?,
			user_uuid: if user_uuid.is_null() {
				None
			} else {
				Some(user_uuid.try_into()?)
			},
			user_name,
			message,
			created_at: created_at.try_into()?,
		})
	}
}
