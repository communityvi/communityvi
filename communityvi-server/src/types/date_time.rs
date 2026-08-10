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

impl From<DateTime> for turso::Value {
	fn from(date_time: DateTime) -> turso::Value {
		turso::Value::Text(date_time.to_rfc3339())
	}
}

impl TryFrom<turso::Value> for DateTime {
	type Error = anyhow::Error;

	fn try_from(value: turso::Value) -> anyhow::Result<Self> {
		let turso::Value::Text(text) = value else {
			return Err(anyhow!("Expected text value"));
		};

		text.parse().map(DateTime).context("Failed to parse DateTime")
	}
}
