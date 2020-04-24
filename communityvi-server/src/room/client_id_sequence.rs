use crate::room::client_id::ClientId;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;

#[derive(Default)]
pub struct ClientIdSequence {
	next_id: AtomicU64,
}

impl ClientIdSequence {
	pub fn next(&self) -> ClientId {
		// Using Relaxed memory ordering is ok because we only care about
		// the ordering of the value in the atomic and not any surrounding
		// loads or stores.
		self.next_id.fetch_add(1, Relaxed).into()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn client_id_sequence_should_count() {
		let sequence = ClientIdSequence::default();
		assert_eq!(ClientId::from(0), sequence.next());
		assert_eq!(ClientId::from(1), sequence.next());
		assert_eq!(ClientId::from(2), sequence.next());
		assert_eq!(ClientId::from(3), sequence.next());
		assert_eq!(ClientId::from(4), sequence.next());
	}
}
