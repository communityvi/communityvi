use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SessionId {
	uuid: Uuid,
}

impl SessionId {
	pub fn new() -> Self {
		Self { uuid: Uuid::new_v4() }
	}
}

impl Display for SessionId {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		self.uuid.fmt(formatter)
	}
}

impl TryFrom<&str> for SessionId {
	type Error = anyhow::Error;

	fn try_from(text: &str) -> anyhow::Result<Self> {
		Ok(Self { uuid: text.parse()? })
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn session_id_can_be_serialized_and_deserialized() {
		let session_id = SessionId::new();

		let serialized = session_id.to_string();
		let deserialized = SessionId::try_from(serialized.as_str()).expect("Failed to deserialize");

		assert_eq!(deserialized, session_id);
	}

	#[test]
	fn session_id_is_randomly_generated() {
		let session_id1 = SessionId::new();
		let session_id2 = SessionId::new();

		assert_ne!(session_id1, session_id2);
	}
}
