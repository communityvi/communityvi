use js_int::UInt;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ClientId(UInt);

impl From<UInt> for ClientId {
	fn from(id: UInt) -> Self {
		ClientId(id)
	}
}

#[cfg(test)]
impl From<u32> for ClientId {
	fn from(id: u32) -> Self {
		ClientId(id.into())
	}
}

impl Display for ClientId {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		write!(formatter, "ClientId({})", self.0)
	}
}
