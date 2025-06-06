#![allow(clippy::should_panic_without_expect)]
use crate::message::outgoing::broadcast_message::{
	BroadcastMessage, ChatBroadcast, ClientJoinedBroadcast, ClientLeftBroadcast, MediumStateChangedBroadcast,
	VersionedMediumBroadcast,
};
use js_int::{UInt, uint};
use std::collections::{BTreeSet, VecDeque};
use tokio::sync::Notify;

pub struct BroadcastBuffer {
	inner: parking_lot::Mutex<Inner>,
	new_broadcast_available_notification_channel: Notify,
	maximum_client_count: usize,
}

const CHAT_MESSAGE_BUFFER_LIMIT: usize = 10;

#[derive(Default)]
pub struct Inner {
	next_medium_version: UInt,
	next_chat_message_counter: UInt,
	messages: VecDeque<BroadcastMessage>,
	next_broadcast_number: Option<usize>,
}

impl BroadcastBuffer {
	pub fn new(maximum_client_count: usize) -> Self {
		Self {
			inner: Default::default(),
			new_broadcast_available_notification_channel: Default::default(),
			maximum_client_count,
		}
	}

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

				inner.next_medium_version = *version + uint!(1);
			}
			BroadcastMessage::Chat(ChatBroadcast { counter, .. }) => {
				if *counter < inner.next_chat_message_counter {
					return;
				}

				inner.next_chat_message_counter = *counter + uint!(1);
			}
			_ => {}
		}

		inner.messages.push_back(message);

		let worst_count_to_keep_alive = self.worst_count_of_messages_to_keep_alive();
		if inner.length() > (worst_count_to_keep_alive + (worst_count_to_keep_alive / 2)) {
			inner.collect_garbage();
		}

		if !inner.is_empty() {
			// FIXME: Check if this use of tokio::sync::Notify is correct!
			self.new_broadcast_available_notification_channel.notify_one();
		}
	}

	fn worst_count_of_messages_to_keep_alive(&self) -> usize {
		(self.maximum_client_count - 1) // join/leave messages for all clients except the one we're currently sending to
			+ (CHAT_MESSAGE_BUFFER_LIMIT * 3) // Join + Chat + Leave if a client Joins, sends a message and leaves again
			+ 3 // Join + medium state + Leave if a client joins, changes the state and leaves again
	}

	pub fn is_empty(&self) -> bool {
		self.inner.lock().is_empty()
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
}

impl Inner {
	/// Ensures that there is a bounded count of messages in the buffer by enforcing some simple rules:
	/// * Only ever keep the medium state with highest version alive
	///   (which is the last in the buffer since the order of versions is already enforced when enqueueing)
	/// * Only ever keep at most the last `CHAT_MESSAGE_BUFFER_LIMIT` chat messages.
	/// * Remove Join and Left messages for the same client as long as we don't still have any chat messages from them.
	///
	/// This means we can calculate the maximum count of messages by taking the worst case scenario
	/// (see `worst_count_of_messages_to_keep_alive`)
	fn collect_garbage(&mut self) {
		let mut seen_chat_messages = 0;
		let mut last_seen_medium_index = None;
		let mut clients_to_keep_alive = BTreeSet::new();
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
					clients_to_keep_alive.insert(*sender_id);
				}
				MediumStateChanged(MediumStateChangedBroadcast { changed_by_id, .. }) => {
					last_seen_medium_index = Some(index);
					clients_to_keep_alive.insert(*changed_by_id);
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
						!left_clients.contains(id) || clients_to_keep_alive.contains(id)
					}
					ClientLeft(ClientLeftBroadcast { id, .. }) => {
						!joined_clients.contains(id) || clients_to_keep_alive.contains(id)
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

	fn length(&self) -> usize {
		self.messages.len()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::message::outgoing::broadcast_message::{LeftReason, MediumBroadcast};
	use crate::room::session_id::SessionId;
	use std::ops::Deref;

	#[allow(clippy::cast_possible_truncation)]
	const CHAT_MESSAGE_BUFFER_LIMIT: u32 = super::CHAT_MESSAGE_BUFFER_LIMIT as _;

	struct BroadcastBufferWithTestHelpers {
		pub broadcast_buffer: BroadcastBuffer,
		pub broadcast_number: usize,
	}

	impl Default for BroadcastBufferWithTestHelpers {
		fn default() -> Self {
			Self {
				broadcast_buffer: BroadcastBuffer::new(50),
				broadcast_number: 0,
			}
		}
	}

	impl Deref for BroadcastBufferWithTestHelpers {
		type Target = BroadcastBuffer;

		fn deref(&self) -> &Self::Target {
			&self.broadcast_buffer
		}
	}

	impl BroadcastBufferWithTestHelpers {
		fn enqueue_next(&mut self, broadcast: BroadcastMessage) {
			let broadcast_number = self.broadcast_number;
			self.broadcast_number += 1;
			self.enqueue(broadcast, broadcast_number);
		}

		fn enqueue_client_joined(&mut self, id: UInt) {
			let message = ClientJoinedBroadcast {
				id: id.into(),
				name: format!("{id}"),
			};
			self.enqueue_next(message.into());
		}

		fn enqueue_client_left(&mut self, id: UInt) {
			let message = ClientLeftBroadcast {
				id: id.into(),
				name: format!("{id}"),
				reason: LeftReason::Closed,
			};
			self.enqueue_next(message.into());
		}

		fn enqueue_medium_state(&mut self, id: SessionId, version: UInt) {
			let medium_state = MediumStateChangedBroadcast {
				changed_by_name: format!("{id}"),
				changed_by_id: id,
				medium: VersionedMediumBroadcast {
					version,
					medium: MediumBroadcast::Empty,
				},
			};
			self.enqueue_next(medium_state.into());
		}

		fn enqueue_chat_message(&mut self, id: SessionId, number: UInt) {
			let chat_message = ChatBroadcast {
				sender_id: id,
				sender_name: format!("{id}"),
				message: format!("{number}"),
				counter: number,
			};
			self.enqueue_next(chat_message.into());
		}

		fn dequeue_client_joined(&mut self) -> SessionId {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::ClientJoined(joined) => joined.id,
				_ => panic!("Head of buffer was not ClientJoined"),
			}
		}

		fn dequeue_client_left(&mut self) -> SessionId {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::ClientLeft(left) => left.id,
				_ => panic!("Head of buffer was not ClientLeft"),
			}
		}

		fn dequeue_medium_state(&mut self) -> (SessionId, UInt) {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::MediumStateChanged(MediumStateChangedBroadcast {
					changed_by_id,
					medium: VersionedMediumBroadcast { version, .. },
					..
				}) => (changed_by_id, version),
				_ => panic!("Head of buffer was not MediumStateChanged"),
			}
		}

		fn dequeue_chat_message(&mut self) -> (SessionId, UInt) {
			match self.broadcast_buffer.dequeue().expect("No message queued") {
				BroadcastMessage::Chat(ChatBroadcast { sender_id, counter, .. }) => (sender_id, counter),
				_ => panic!("Head of buffer was not Chat"),
			}
		}
	}

	#[test]
	fn collect_garbage_should_remove_pairs_of_client_messages() {
		let mut broadcast_buffer = BroadcastBufferWithTestHelpers::default();
		broadcast_buffer.enqueue_client_joined(uint!(0));
		broadcast_buffer.enqueue_client_left(uint!(0));
		broadcast_buffer.enqueue_client_joined(uint!(1));
		broadcast_buffer.enqueue_client_joined(uint!(2));
		broadcast_buffer.enqueue_client_left(uint!(99));
		broadcast_buffer.enqueue_client_left(uint!(1));

		broadcast_buffer.inner.lock().collect_garbage();

		assert_eq!(broadcast_buffer.dequeue_client_joined(), SessionId::from(2));
		assert_eq!(broadcast_buffer.dequeue_client_left(), SessionId::from(99));
		assert!(broadcast_buffer.is_empty());
	}

	#[test]
	fn collect_garbage_should_only_produce_latest_medium_state() {
		let mut broadcast_buffer = BroadcastBufferWithTestHelpers::default();
		broadcast_buffer.enqueue_medium_state(SessionId::from(42), uint!(13));
		broadcast_buffer.enqueue_medium_state(SessionId::from(12), uint!(14));
		broadcast_buffer.enqueue_medium_state(SessionId::from(1), uint!(1));

		broadcast_buffer.inner.lock().collect_garbage();

		let (id, version) = broadcast_buffer.dequeue_medium_state();
		assert_eq!(id, SessionId::from(12));
		assert_eq!(version, uint!(14));
	}

	#[test]
	fn should_not_store_more_than_limit_chat_messages() {
		let mut broadcast_buffer = BroadcastBufferWithTestHelpers::default();
		for number in 0..(CHAT_MESSAGE_BUFFER_LIMIT + 3) {
			broadcast_buffer.enqueue_chat_message(SessionId::from(number), number.into());
		}

		broadcast_buffer.inner.lock().collect_garbage();

		for number in 3..(CHAT_MESSAGE_BUFFER_LIMIT + 3) {
			let (id, count) = broadcast_buffer.dequeue_chat_message();
			assert_eq!(id, SessionId::from(number));
			assert_eq!(count, UInt::from(number));
		}
	}

	#[test]
	fn chat_messages_should_keep_clients_alive() {
		let mut broadcast_buffer = BroadcastBufferWithTestHelpers::default();
		broadcast_buffer.enqueue_client_joined(uint!(42));
		broadcast_buffer.enqueue_chat_message(SessionId::from(42), uint!(1337));
		broadcast_buffer.enqueue_client_left(uint!(42));

		broadcast_buffer.inner.lock().collect_garbage();

		assert_eq!(broadcast_buffer.dequeue_client_joined(), SessionId::from(42));
		let (id, count) = broadcast_buffer.dequeue_chat_message();
		assert_eq!(id, SessionId::from(42));
		assert_eq!(count, uint!(1337));
		assert_eq!(broadcast_buffer.dequeue_client_left(), SessionId::from(42));
	}

	#[test]
	fn medium_state_messages_should_keep_clients_alive() {
		let mut broadcast_buffer = BroadcastBufferWithTestHelpers::default();
		broadcast_buffer.enqueue_client_joined(uint!(42));
		broadcast_buffer.enqueue_medium_state(SessionId::from(42), uint!(2));
		broadcast_buffer.enqueue_client_left(uint!(42));

		broadcast_buffer.inner.lock().collect_garbage();

		assert_eq!(broadcast_buffer.dequeue_client_joined(), SessionId::from(42));
		let (id, version) = broadcast_buffer.dequeue_medium_state();
		assert_eq!(id, SessionId::from(42));
		assert_eq!(version, uint!(2));
		assert_eq!(broadcast_buffer.dequeue_client_left(), SessionId::from(42));
	}

	#[test]
	#[should_panic]
	fn broadcast_number_must_not_stay_the_same() {
		let broadcast_buffer = BroadcastBufferWithTestHelpers::default();

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
		let broadcast_buffer = BroadcastBufferWithTestHelpers::default();

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
		let broadcast_buffer = BroadcastBufferWithTestHelpers::default();

		let message = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
			id: 0.into(),
			name: String::default(),
		});
		broadcast_buffer.enqueue(message.clone(), 42);
		broadcast_buffer.enqueue(message, 41);
	}

	#[allow(clippy::cast_possible_truncation)]
	#[test]
	fn should_trigger_garbage_collection_after_one_and_a_half_time_worst_count_is_exceeded() {
		let mut broadcast_buffer = BroadcastBufferWithTestHelpers::default();

		let worst_count_to_keep_alive = broadcast_buffer.worst_count_of_messages_to_keep_alive() as u32;
		for number in 0..(worst_count_to_keep_alive + (worst_count_to_keep_alive / 2)) {
			broadcast_buffer.enqueue_medium_state(SessionId::from(number), number.into());
		}

		assert_eq!(
			broadcast_buffer.inner.lock().length() as u32,
			(worst_count_to_keep_alive + (worst_count_to_keep_alive / 2))
		);

		broadcast_buffer.enqueue_medium_state(SessionId::from(1337), uint!(1337));
		assert_eq!(broadcast_buffer.inner.lock().length(), 1); // garbage collection should have been triggered
	}
}
