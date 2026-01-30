use anyhow::{Context, anyhow};
use chrono::Utc;

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
