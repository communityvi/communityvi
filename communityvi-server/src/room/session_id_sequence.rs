use crate::room::session_id::SessionId;
use js_int::UInt;
use std::ops::RangeInclusive;

pub struct SessionIdSequence {
	id_pool: RangeInclusive<u64>,
}

impl Default for SessionIdSequence {
	fn default() -> Self {
		Self {
			id_pool: UInt::MIN.into()..=UInt::MAX.into(),
		}
	}
}

impl SessionIdSequence {
	pub fn next(&mut self) -> SessionId {
		SessionId::from(
			UInt::try_from(self.id_pool.next().expect("Ran out of available ClientIds."))
				.unwrap_or_else(|_| unreachable!()),
		)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn session_id_sequence_should_increment() {
		let mut sequence = SessionIdSequence::default();
		assert_eq!(SessionId::from(0), sequence.next());
		assert_eq!(SessionId::from(1), sequence.next());
		assert_eq!(SessionId::from(2), sequence.next());
		assert_eq!(SessionId::from(3), sequence.next());
		assert_eq!(SessionId::from(4), sequence.next());
	}
}
