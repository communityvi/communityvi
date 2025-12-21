use anyhow::{Context, anyhow};
use chrono::Utc;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::{Database, Decode, Encode, Type};

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
pub struct DateTime(chrono::DateTime<Utc>);

impl<'r, DB> Decode<'r, DB> for DateTime
where
	DB: Database,
	chrono::DateTime<Utc>: Decode<'r, DB>,
{
	fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let date_time = chrono::DateTime::decode(value)?;
		Ok(Self(date_time))
	}
}

impl<'r, DB> Encode<'r, DB> for DateTime
where
	DB: Database,
	chrono::DateTime<Utc>: Encode<'r, DB>,
{
	fn encode_by_ref(&self, buffer: &mut <DB as Database>::ArgumentBuffer<'r>) -> Result<IsNull, BoxDynError> {
		self.0.encode_by_ref(buffer)
	}
}

impl<DB> Type<DB> for DateTime
where
	DB: Database,
	chrono::DateTime<Utc>: Type<DB>,
{
	fn type_info() -> DB::TypeInfo {
		chrono::DateTime::<Utc>::type_info()
	}

	fn compatible(ty: &DB::TypeInfo) -> bool {
		chrono::DateTime::<Utc>::compatible(ty)
	}
}

impl From<DateTime> for libsql::Value {
	fn from(date_time: DateTime) -> libsql::Value {
		libsql::Value::Text(date_time.to_rfc3339())
	}
}

impl TryFrom<libsql::Value> for DateTime {
	type Error = anyhow::Error;

	fn try_from(value: libsql::Value) -> anyhow::Result<Self> {
		let libsql::Value::Text(text) = value else {
			return Err(anyhow!("Expected text value"));
		};

		text.parse().map(DateTime).context("Failed to parse DateTime")
	}
}
