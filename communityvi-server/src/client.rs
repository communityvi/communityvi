use crate::message::{OrderedMessage, ServerResponse};
use futures::channel::mpsc::{SendError, Sender};
use futures::SinkExt;
use log::info;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct Client {
	id: usize,
	sender: Sender<OrderedMessage<ServerResponse>>,
}

impl Client {
	pub fn new(id: usize, sender: Sender<OrderedMessage<ServerResponse>>) -> Self {
		Self { id, sender }
	}

	pub fn id(&self) -> usize {
		self.id
	}

	pub async fn send(&self, message: OrderedMessage<ServerResponse>) -> Result<(), ()> {
		let send_result = self.sender.clone().send(message).await;
		send_result.map_err(|_: SendError| {
			info!("Client with id {} has gone away.", self.id);
		})
	}
}

impl Hash for Client {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state)
	}
}

impl PartialEq for Client {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Eq for Client {}
