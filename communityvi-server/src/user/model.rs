use crate::types::uuid::Uuid;
use sqlx::FromRow;

#[derive(FromRow, Clone, Debug, PartialEq, Eq)]
pub struct User {
	pub uuid: Uuid,
	pub name: String,
	pub normalized_name: String,
}

impl TryFrom<libsql::Row> for User {
	type Error = anyhow::Error;

	fn try_from(row: libsql::Row) -> anyhow::Result<Self> {
		let uuid = row.get_value(0)?;
		let name = row.get(1)?;
		let normalized_name = row.get(2)?;

		Ok(Self {
			uuid: uuid.try_into()?,
			name,
			normalized_name,
		})
	}
}
