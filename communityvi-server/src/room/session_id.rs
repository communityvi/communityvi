use js_int::UInt;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct SessionId(UInt);

impl From<UInt> for SessionId {
	fn from(id: UInt) -> Self {
		SessionId(id)
	}
}

#[cfg(test)]
impl From<u32> for SessionId {
	fn from(id: u32) -> Self {
		SessionId(id.into())
	}
}

impl Display for SessionId {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		write!(formatter, "ClientId({})", self.0)
	}
}
