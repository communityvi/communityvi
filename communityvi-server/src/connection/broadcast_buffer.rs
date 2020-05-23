use crate::message::outgoing::broadcast_message::{
	BroadcastMessage, ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast, MediumStateChangedBroadcast,
	VersionedMediumBroadcast,
};
use std::collections::{BTreeSet, VecDeque};
use tokio::sync::Notify;

#[derive(Default)]
pub struct BroadcastBuffer {
	inner: parking_lot::Mutex<Inner>,
	new_broadcast_available_notification_channel: Notify,
}

const CHAT_MESSAGE_BUFFER_LIMIT: usize = 10;

#[derive(Default)]
pub struct Inner {
	next_medium_version: u64,
	next_chat_message_counter: u64,
	messages: VecDeque<BroadcastMessage>,
	next_broadcast_number: Option<usize>,
}

impl BroadcastBuffer {
	pub fn enqueue(&self, message: BroadcastMessage, broadcast_number: usize) {
		let mut inner = self.inner.lock();
		if let Some(next_broadcast_number) = inner.next_broadcast_number {
			assert_eq!(next_broadcast_number, broadcast_number);
		}
		inner.next_broadcast_number = Some(broadcast_number + 1);

		match &message {
			BroadcastMessage::MediumStateChanged(MediumStateChangedBroadcast {
				medium: VersionedMediumBroadcast { version, .. },
				..
			}) => {
				if *version < inner.next_medium_version {
					return;
				}

				inner.next_medium_version = version + 1;
			}
			BroadcastMessage::Chat(ChatBroadcast { counter, .. }) => {
				if *counter < inner.next_chat_message_counter {
					return;
				}

				inner.next_chat_message_counter = counter + 1;
			}
			_ => {}
		}

		inner.messages.push_back(message);
		inner.collect_garbage(); // TODO: Decide when to actually collect.

		if !inner.is_empty() {
			self.new_broadcast_available_notification_channel.notify();
		}
	}

	pub fn dequeue(&self) -> Option<BroadcastMessage> {
		self.inner.lock().messages.pop_front()
	}

	pub async fn wait_for_broadcast(&self) -> BroadcastMessage {
		loop {
			self.new_broadcast_available_notification_channel.notified().await;
			if let Some(broadcast) = self.dequeue() {
				return broadcast;
			}
		}
	}

	pub fn is_empty(&self) -> bool {
		self.inner.lock().is_empty()
	}
}

impl Inner {
	/// Ensures that there is a bounded count of messages in the buffer by enforcing some simple rules:
	/// * Only ever keep the medium state with highest version alive
	///		 (which is the last in the buffer since the order of versions is already enforced when enqueueing)
	/// * Only ever keep at most the last CHAT_MESSAGE_BUFFER_LIMIT chat messages.
	/// * Remove Join and Left messages for the same client as long as we don't still have any chat messages from them.
	///
	/// This means we can calculate the maximum count of messages by taking the worst case scenario:
	/// * (room_size_limit - 1) regular client join messages (not including the client this buffer is for)
	/// * (CHAT_MESSAGE_BUFFER_LIMIT * 2) Join/Leave messages (if they join, then send one message, then leave again)
	/// * 1 medium state message (with the highest version)
	fn collect_garbage(&mut self) {
		let mut seen_chat_messages = 0;
		let mut last_seen_medium_index = None;
		let mut seen_chatters = BTreeSet::new();
		let mut joined_clients = BTreeSet::new();
		let mut left_clients = BTreeSet::new();

		// Mark phase
		for (index, message) in self.messages.iter().enumerate() {
			use BroadcastMessage::*;
			match message {
				ClientJoined(ClientJoinedBroadcast { id, .. }) => {
					joined_clients.insert(*id);
				}
				ClientLeft(ClientLeftBroadcast { id, .. }) => {
					left_clients.insert(*id);
				}
				Chat(ChatBroadcast { sender_id, .. }) => {
					seen_chat_messages += 1;
					seen_chatters.insert(*sender_id);
				}
				MediumStateChanged(_) => {
					last_seen_medium_index = Some(index);
				}
			}
		}

		// Sweep Phase
		self.messages = self
			.messages
			.drain(..)
			.enumerate()
			.filter(|(index, message)| {
				use BroadcastMessage::*;
				match message {
					ClientJoined(ClientJoinedBroadcast { id, .. }) => {
						!left_clients.contains(id) || seen_chatters.contains(id)
					}
					ClientLeft(ClientLeftBroadcast { id, .. }) => {
						!joined_clients.contains(id) || seen_chatters.contains(id)
					}
					Chat(_) => {
						let keep = seen_chat_messages <= CHAT_MESSAGE_BUFFER_LIMIT;
						seen_chat_messages -= 1;
						keep
					}
					MediumStateChanged(_) => Some(*index) == last_seen_medium_index,
				}
			})
			.map(|(_index, message)| message)
			.collect();
	}

	fn is_empty(&self) -> bool {
		self.messages.is_empty()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::message::outgoing::broadcast_message::MediumBroadcast;
	use crate::room::client_id::ClientId;
	use crate::utils::backtrace_disabler::BacktraceDisabler;
	use std::ops::Deref;

	#[derive(Default)]
	struct BroadCastBufferWithTestHelpers {
		pub broadcast_buffer: BroadcastBuffer,
		pub broadcast_number: usize,
	}

	impl Deref for BroadCastBufferWithTestHelpers {
		type Target = BroadcastBuffer;

		fn deref(&self) -> &Self::Target {
			&self.broadcast_buffer
		}
	}

	impl BroadCastBufferWithTestHelpers {
		fn enqueue_next(&mut self, broadcast: BroadcastMessage) {
			let broadcast_number = self.broadcast_number;
			self.broadcast_number += 1;
			self.enqueue(broadcast, broadcast_number);
		}

		fn enqueue_client_joined(&mut self, id: u64) {
			let message = ClientJoinedBroadcast {
				id: id.into(),
				name: format!("{}", id),
			};
			self.enqueue_next(message.into());
		}

		fn enqueue_client_left(&mut self, id: u64) {
			let message = ClientLeftBroadcast {
				id: id.into(),
				name: format!("{}", id),
			};
			self.enqueue_next(message.into());
		}

		fn enqueue_medium_state(&mut self, id: ClientId, version: u64) {
			let medium_state = MediumStateChangedBroadcast {
				changed_by_name: format!("{}", id),
				changed_by_id: id,
				medium: VersionedMediumBroadcast {
					version,
					medium: MediumBroadcast::Empty,
				},
			};
			self.enqueue_next(medium_state.into());
		}

		fn enqueue_chat_message(&mut self, id: ClientId, number: u64) {
			let chat_message = ChatBroadcast {
				sender_id: id,
				sender_name: format!("{}", id),
				message: format!("{}", number),
				counter: number,
			};
			self.enqueue_next(chat_message.into());
		}

		fn dequeue_client_joined(&mut self) -> u64 {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::ClientJoined(joined) => joined.id.into(),
				_ => panic!("Head of buffer was not ClientJoined"),
			}
		}

		fn dequeue_client_left(&mut self) -> u64 {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::ClientLeft(left) => left.id.into(),
				_ => panic!("Head of buffer was not ClientLeft"),
			}
		}

		fn dequeue_medium_state(&mut self) -> (ClientId, u64) {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::MediumStateChanged(MediumStateChangedBroadcast {
					changed_by_id,
					medium: VersionedMediumBroadcast { version, .. },
					..
				}) => (changed_by_id, version),
				_ => panic!("Head of buffer was not MediumStateChanged"),
			}
		}

		fn dequeue_chat_message(&mut self) -> (ClientId, u64) {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::Chat(ChatBroadcast { sender_id, counter, .. }) => (sender_id, counter),
				_ => panic!("Head of buffer was not Chat"),
			}
		}
	}

	#[test]
	fn collect_garbage_should_remove_pairs_of_client_messages() {
		let mut broadcast_buffer = BroadCastBufferWithTestHelpers::default();
		broadcast_buffer.enqueue_client_joined(0);
		broadcast_buffer.enqueue_client_left(0);
		broadcast_buffer.enqueue_client_joined(1);
		broadcast_buffer.enqueue_client_joined(2);
		broadcast_buffer.enqueue_client_left(99);
		broadcast_buffer.enqueue_client_left(1);

		assert_eq!(broadcast_buffer.dequeue_client_joined(), 2);
		assert_eq!(broadcast_buffer.dequeue_client_left(), 99);
		assert!(broadcast_buffer.is_empty());
	}

	#[test]
	fn collect_garbage_should_only_produce_latest_medium_state() {
		let mut broadcast_buffer = BroadCastBufferWithTestHelpers::default();
		broadcast_buffer.enqueue_medium_state(ClientId::from(42), 13);
		broadcast_buffer.enqueue_medium_state(ClientId::from(12), 14);
		broadcast_buffer.enqueue_medium_state(ClientId::from(1), 1);

		let (id, version) = broadcast_buffer.dequeue_medium_state();
		assert_eq!(id, ClientId::from(12));
		assert_eq!(version, 14);
	}

	#[test]
	fn should_not_store_more_than_limit_chat_messages() {
		let mut broadcast_buffer = BroadCastBufferWithTestHelpers::default();
		for number in 0..(CHAT_MESSAGE_BUFFER_LIMIT as u64 + 3) {
			broadcast_buffer.enqueue_chat_message(ClientId::from(number), number);
		}

		for number in 3..(CHAT_MESSAGE_BUFFER_LIMIT as u64 + 3) {
			let (id, count) = broadcast_buffer.dequeue_chat_message();
			assert_eq!(id, ClientId::from(number));
			assert_eq!(count, number);
		}
	}

	#[test]
	fn chat_messages_should_keep_clients_alive() {
		let mut broadcast_buffer = BroadCastBufferWithTestHelpers::default();
		broadcast_buffer.enqueue_client_joined(42);
		broadcast_buffer.enqueue_chat_message(ClientId::from(42), 1337);
		broadcast_buffer.enqueue_client_left(42);

		assert_eq!(broadcast_buffer.dequeue_client_joined(), 42);
		let (id, count) = broadcast_buffer.dequeue_chat_message();
		assert_eq!(id, ClientId::from(42));
		assert_eq!(count, 1337);
		assert_eq!(broadcast_buffer.dequeue_client_left(), 42);
	}

	#[test]
	#[should_panic]
	fn broadcast_number_must_not_stay_the_same() {
		let _backtrace_disabler = BacktraceDisabler::default();
		let broadcast_buffer = BroadCastBufferWithTestHelpers::default();

		let message = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
			id: 0.into(),
			name: String::default(),
		});
		broadcast_buffer.enqueue(message.clone(), 42);
		broadcast_buffer.enqueue(message, 42);
	}

	#[test]
	#[should_panic]
	fn broadcast_number_must_not_skip() {
		let _backtrace_disabler = BacktraceDisabler::default();
		let broadcast_buffer = BroadCastBufferWithTestHelpers::default();

		let message = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
			id: 0.into(),
			name: String::default(),
		});
		broadcast_buffer.enqueue(message.clone(), 42);
		broadcast_buffer.enqueue(message, 44);
	}

	#[test]
	#[should_panic]
	fn broadcast_number_must_not_decrease() {
		let _backtrace_disabler = BacktraceDisabler::default();
		let broadcast_buffer = BroadCastBufferWithTestHelpers::default();

		let message = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
			id: 0.into(),
			name: String::default(),
		});
		broadcast_buffer.enqueue(message.clone(), 42);
		broadcast_buffer.enqueue(message, 41);
	}
}
