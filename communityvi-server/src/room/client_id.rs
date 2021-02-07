use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ClientId {
	id: u64,
}

impl From<u64> for ClientId {
	fn from(id: u64) -> Self {
		ClientId { id }
	}
}

impl Into<u64> for ClientId {
	fn into(self) -> u64 {
		self.id
	}
}

impl Display for ClientId {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		write!(formatter, "ClientId({})", self.id)
	}
}
