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

impl From<u32> for ClientId {
	fn from(id: u32) -> Self {
		ClientId(id.into())
	}
}

impl From<ClientId> for UInt {
	fn from(ClientId(id): ClientId) -> Self {
		id
	}
}

impl From<ClientId> for u64 {
	fn from(ClientId(id): ClientId) -> Self {
		id.into()
	}
}

impl TryFrom<u64> for ClientId {
	type Error = anyhow::Error;

	fn try_from(id: u64) -> anyhow::Result<Self> {
		Ok(ClientId(UInt::try_from(id)?))
	}
}

impl Display for ClientId {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		write!(formatter, "ClientId({})", self.0)
	}
}
