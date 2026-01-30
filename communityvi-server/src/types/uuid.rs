use anyhow::{Context, anyhow};
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
