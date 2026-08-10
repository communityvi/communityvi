use crate::types::uuid::Uuid;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
	pub uuid: Uuid,
	pub name: String,
	pub normalized_name: String,
}

impl TryFrom<turso::Row> for User {
	type Error = anyhow::Error;

	fn try_from(row: turso::Row) -> anyhow::Result<Self> {
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
