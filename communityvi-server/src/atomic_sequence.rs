use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::SeqCst;

#[derive(Default)]
pub struct AtomicSequence {
	next_number: AtomicU64,
}

impl AtomicSequence {
	pub fn next(&self) -> u64 {
		self.next_number.fetch_add(1, SeqCst)
	}
}

#[cfg(test)]
mod test {
	use crate::atomic_sequence::AtomicSequence;

	#[test]
	fn atomic_sequence_should_count() {
		let sequence = AtomicSequence::default();
		assert_eq!(0, sequence.next());
		assert_eq!(1, sequence.next());
		assert_eq!(2, sequence.next());
		assert_eq!(3, sequence.next());
		assert_eq!(4, sequence.next());
	}
}
