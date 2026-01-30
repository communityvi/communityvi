use crate::types::uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
	pub uuid: Uuid,
	pub name: String,
	pub medium_uuid: Option<Uuid>,
}

impl TryFrom<libsql::Row> for Room {
	type Error = anyhow::Error;

	fn try_from(row: libsql::Row) -> Result<Self, Self::Error> {
		let uuid = row.get_value(0)?;
		let name = row.get(1)?;
		let medium_uuid = row.get_value(2)?;

		Ok(Self {
			uuid: uuid.try_into()?,
			name,
			medium_uuid: if medium_uuid.is_null() {
				None
			} else {
				Some(medium_uuid.try_into()?)
			},
		})
	}
}
