use anyhow::{Context, anyhow};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::{Database, Decode, Encode};

#[derive(
	derive_more::From,
	derive_more::Into,
	derive_more::Deref,
	derive_more::DerefMut,
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
)]
pub struct Uuid(uuid::Uuid);

impl Uuid {
	pub fn new_v4() -> Self {
		Self(uuid::Uuid::new_v4())
	}
}

impl<'r, Db> Decode<'r, Db> for Uuid
where
	Db: Database,
	uuid::Uuid: Decode<'r, Db>,
{
	fn decode(value: <Db as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		uuid::Uuid::decode(value).map(Uuid)
	}
}

impl<'q, Db> Encode<'q, Db> for Uuid
where
	Db: Database,
	uuid::Uuid: Encode<'q, Db>,
{
	fn encode_by_ref(&self, buffer: &mut <Db as Database>::ArgumentBuffer<'q>) -> Result<IsNull, BoxDynError> {
		self.0.encode_by_ref(buffer)
	}
}

impl<Db> sqlx::Type<Db> for Uuid
where
	Db: Database,
	uuid::Uuid: sqlx::Type<Db>,
{
	fn type_info() -> Db::TypeInfo {
		uuid::Uuid::type_info()
	}

	fn compatible(type_info: &Db::TypeInfo) -> bool {
		uuid::Uuid::compatible(type_info)
	}
}

impl From<Uuid> for libsql::Value {
	fn from(Uuid(uuid): Uuid) -> libsql::Value {
		libsql::Value::Text(uuid.to_string())
	}
}

impl TryFrom<libsql::Value> for Uuid {
	type Error = anyhow::Error;
	fn try_from(value: libsql::Value) -> anyhow::Result<Self> {
		let libsql::Value::Text(text) = value else {
			return Err(anyhow!("Expected text value"));
		};

		text.parse().map(Uuid).context("Failed to parse UUID")
	}
}
