use crate::connection::client::ClientConnection;
use crate::message::ServerResponse;
use log::info;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct Client {
	id: ClientId,
	name: String,
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

	pub async fn send(&self, message: ServerResponse) -> Result<(), ()> {
		let send_result = self.connection.send(message).await;
		send_result.map_err(|_: ()| {
			info!("Client with id {} has gone away.", self.id);
		})
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
