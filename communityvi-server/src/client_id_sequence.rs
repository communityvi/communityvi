use crate::atomic_sequence::AtomicSequence;
use crate::client::ClientId;

#[derive(Default)]
pub struct ClientIdSequence {
	sequence: AtomicSequence,
}

impl ClientIdSequence {
	pub fn next(&self) -> ClientId {
		self.sequence.next().into()
	}
}
