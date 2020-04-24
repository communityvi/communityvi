use crate::connection::client::ClientConnection;
use debug_stub_derive::DebugStub;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;

#[derive(DebugStub)]
pub struct Client {
	pub name: String,
	#[debug_stub = "ClientConnection"]
	pub connection: ClientConnection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
