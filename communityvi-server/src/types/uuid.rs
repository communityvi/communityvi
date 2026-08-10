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

impl From<Uuid> for turso::Value {
	fn from(Uuid(uuid): Uuid) -> turso::Value {
		turso::Value::Text(uuid.to_string())
	}
}

impl TryFrom<turso::Value> for Uuid {
	type Error = anyhow::Error;
	fn try_from(value: turso::Value) -> anyhow::Result<Self> {
		let turso::Value::Text(text) = value else {
			return Err(anyhow!("Expected text value"));
		};

		text.parse().map(Uuid).context("Failed to parse UUID")
	}
}
