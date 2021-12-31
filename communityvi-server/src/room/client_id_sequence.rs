use crate::room::client_id::ClientId;
use js_int::UInt;
use std::ops::RangeInclusive;

pub struct ClientIdSequence {
	id_pool: RangeInclusive<u64>,
}

impl Default for ClientIdSequence {
	fn default() -> Self {
		Self {
			id_pool: UInt::MIN.into()..=UInt::MAX.into(),
		}
	}
}

impl ClientIdSequence {
	pub fn next(&mut self) -> ClientId {
		ClientId::from(
			UInt::try_from(self.id_pool.next().expect("Ran out of available ClientIds."))
				.unwrap_or_else(|_| unreachable!()),
		)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn client_id_sequence_should_count() {
		let mut sequence = ClientIdSequence::default();
		assert_eq!(ClientId::from(0), sequence.next());
		assert_eq!(ClientId::from(1), sequence.next());
		assert_eq!(ClientId::from(2), sequence.next());
		assert_eq!(ClientId::from(3), sequence.next());
		assert_eq!(ClientId::from(4), sequence.next());
	}
}
