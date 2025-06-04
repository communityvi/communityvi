use js_int::UInt;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

#[derive(Default)]
pub struct AtomicUInt(AtomicU64);

impl AtomicUInt {
	pub fn new(value: UInt) -> Self {
		Self(AtomicU64::new(value.into()))
	}

	pub fn load(&self, ordering: Ordering) -> UInt {
		assert_in_range_of_uint(self.0.load(ordering))
	}

	/// Increment by the given value, returning the previous value either in Ok if it succeeds or Err if it would go
	/// out of range of [`UInt`].
	///
	/// Takes two memory orderings because it needs to use compare and swap internally to uphold
	/// the invariants of [`UInt`].
	pub fn increment(&self, by: UInt, set_ordering: Ordering, fetch_ordering: Ordering) -> Result<UInt, UInt> {
		self.0
			.fetch_update(set_ordering, fetch_ordering, |value| {
				let new_value = value.checked_add(u64::from(by))?;
				(new_value <= UInt::MAX.into()).then_some(new_value)
			})
			.map(assert_in_range_of_uint)
			.map_err(assert_in_range_of_uint)
	}

	/// Decrement by the given value, returning the previous value either in Ok if it succeeds or Err if it would go
	/// out of range of [`UInt`].
	///
	/// Takes two memory orderings because it needs to use compare and swap internally to uphold
	/// the invariants of [`UInt`].
	pub fn decrement(&self, by: UInt, set_ordering: Ordering, fetch_ordering: Ordering) -> Result<UInt, UInt> {
		self.0
			.fetch_update(set_ordering, fetch_ordering, |value| value.checked_sub(u64::from(by)))
			.map(assert_in_range_of_uint)
			.map_err(assert_in_range_of_uint)
	}
}

fn assert_in_range_of_uint(value: u64) -> UInt {
	UInt::new(value).unwrap_or_else(|| unreachable!("AtomicUInt checks the invariant that the u64 fits into a UInt"))
}

#[cfg(test)]
mod tests {
	use super::*;
	use js_int::uint;

	#[test]
	fn increments() {
		let value = AtomicUInt::new(uint!(1337));

		let previous = value.increment(uint!(42), Ordering::Relaxed, Ordering::Relaxed);

		let new_value = value.load(Ordering::Relaxed);
		assert_eq!(Ok(uint!(1337)), previous);
		assert_eq!(uint!(1379), new_value);
	}

	#[test]
	fn decrements() {
		let value = AtomicUInt::new(uint!(1337));

		let previous = value.decrement(uint!(42), Ordering::Relaxed, Ordering::Relaxed);

		let new_value = value.load(Ordering::Relaxed);
		assert_eq!(Ok(uint!(1337)), previous);
		assert_eq!(uint!(1295), new_value);
	}

	#[test]
	fn doesnt_increment_past_max() {
		let value = AtomicUInt::new(UInt::MAX);

		let previous = value.increment(uint!(1), Ordering::Relaxed, Ordering::Relaxed);

		assert_eq!(Err(UInt::MAX), previous);
	}

	#[test]
	fn doesnt_decrement_past_zero() {
		let value = AtomicUInt::new(uint!(10));

		let previous = value.decrement(uint!(11), Ordering::Relaxed, Ordering::Relaxed);

		assert_eq!(Err(uint!(10)), previous);
	}

	#[test]
	fn loads_value() {
		let value = AtomicUInt::new(uint!(42));

		let loaded = value.load(Ordering::Relaxed);

		assert_eq!(uint!(42), loaded);
	}
}
