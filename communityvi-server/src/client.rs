use crate::message::{OrderedMessage, ServerResponse};
use futures::channel::mpsc::{SendError, Sender};
use futures::SinkExt;
use log::info;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct Client {
	id: ClientId,
	sender: Sender<OrderedMessage<ServerResponse>>,
}

impl Client {
	pub fn new(id: ClientId, sender: Sender<OrderedMessage<ServerResponse>>) -> Self {
		Self { id, sender }
	}

	pub fn id(&self) -> ClientId {
		self.id
	}

	pub async fn send(&self, message: OrderedMessage<ServerResponse>) -> Result<(), ()> {
		let send_result = self.sender.clone().send(message).await;
		send_result.map_err(|_: SendError| {
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
