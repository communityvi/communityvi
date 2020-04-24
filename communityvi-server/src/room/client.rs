use crate::connection::client::ClientConnection;
use debug_stub_derive::DebugStub;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;

#[derive(DebugStub)]
pub struct Client {
	id: ClientId,
	name: String,
	#[debug_stub = "ClientConnection"]
	connection: ClientConnection,
}

impl Client {
	pub fn new(id: ClientId, name: String, connection: ClientConnection) -> Self {
		Self { id, name, connection }
	}

	pub fn id(&self) -> ClientId {
		self.id
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn connection(&self) -> ClientConnection {
		self.connection.clone()
	}
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
