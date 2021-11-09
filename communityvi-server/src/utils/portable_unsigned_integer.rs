use serde::Serialize;
use std::error::Error;
use std::fmt::{Display, Formatter};

mod deserialize;

/// Integer that works across modern programming languages, even those that only
/// support IEE754 Double Precision Floating Point numbers like JavaScript.
#[derive(Serialize, PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct PortableUnsignedInteger(u64);

impl PortableUnsignedInteger {
	pub const MAX: u64 = (2u64.pow(53) - 1);
	pub const MIN: u64 = 0;
}

impl From<u32> for PortableUnsignedInteger {
	fn from(number: u32) -> Self {
		Self(number.into())
	}
}

impl From<u16> for PortableUnsignedInteger {
	fn from(number: u16) -> Self {
		Self(number.into())
	}
}

impl From<u8> for PortableUnsignedInteger {
	fn from(number: u8) -> Self {
		Self(number.into())
	}
}

impl TryFrom<u64> for PortableUnsignedInteger {
	type Error = InvalidNumber;

	fn try_from(number: u64) -> Result<Self, Self::Error> {
		if !(PortableUnsignedInteger::MIN..PortableUnsignedInteger::MAX).contains(&number) {
			return Err(InvalidNumber::new(number));
		}

		Ok(PortableUnsignedInteger(number))
	}
}

impl TryFrom<f64> for PortableUnsignedInteger {
	type Error = InvalidNumber;

	fn try_from(number: f64) -> Result<Self, Self::Error> {
		if !number.is_normal() || number.is_sign_negative() || number.fract() != 0.0 {
			return Err(InvalidNumber::new(number));
		}

		// truncation is explicit and assured to work since
		// we checked that the double value is "normal", meaning
		// neither NaN, infinite or "subnormal" (which are never integers)
		#[allow(clippy::cast_possible_truncation)]
		// we already checked that the number is not negative
		#[allow(clippy::cast_sign_loss)]
		let integer = number.trunc() as u64;
		Self::try_from(integer)
	}
}

impl From<PortableUnsignedInteger> for u64 {
	fn from(PortableUnsignedInteger(number): PortableUnsignedInteger) -> Self {
		number
	}
}

impl Display for PortableUnsignedInteger {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		self.0.fmt(formatter)
	}
}

#[derive(Debug)]
pub struct InvalidNumber {
	number: String,
}

impl InvalidNumber {
	fn new(number: impl Display) -> Self {
		Self {
			number: number.to_string(),
		}
	}
}

impl Display for InvalidNumber {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		write!(
			formatter,
			"Expected integer between 0 and 2^53. Got {} instead.",
			self.number
		)
	}
}

impl Error for InvalidNumber {}

#[cfg(test)]
mod test {
	use crate::utils::portable_unsigned_integer::PortableUnsignedInteger;

	#[test]
	fn portable_integer_can_be_serialized() {
		let number = PortableUnsignedInteger(42);

		let json = serde_json::to_string(&number).expect("Failed to serialize number.");
		assert_eq!("42", json);
	}

	#[test]
	fn portable_integer_can_be_created_from_u64() {
		let number = PortableUnsignedInteger::try_from(42u64).expect("Failed to create from u64");
		assert_eq!(PortableUnsignedInteger(42), number);
	}

	#[test]
	fn portable_integer_cannot_be_created_from_u64_out_of_range() {
		PortableUnsignedInteger::try_from(u64::MAX).expect_err("Creation from u64::MAX should have failed but didn't");
	}

	#[test]
	fn portable_integer_can_be_created_from_f64() {
		let number = PortableUnsignedInteger::try_from(42.0).expect("Failed to create from f64");
		assert_eq!(PortableUnsignedInteger(42), number);
	}

	#[test]
	fn portable_integer_cannot_be_created_from_incompatible_f64() {
		PortableUnsignedInteger::try_from(-1.0)
			.expect_err("Creation from negative number should have failed but didn't");
		PortableUnsignedInteger::try_from(0.5)
			.expect_err("Creation from fractional number should have failed but didn't");
		PortableUnsignedInteger::try_from(f64::MAX).expect_err("Creation from f64::MAX should have failed but didn't");
		PortableUnsignedInteger::try_from(f64::NAN).expect_err("Creation from NaN should have failed but didn't");
		const SUBNORMAL_NUMBER: f64 = 1.0e-308_f64;
		assert!(SUBNORMAL_NUMBER.is_subnormal());
		PortableUnsignedInteger::try_from(SUBNORMAL_NUMBER)
			.expect_err("Creation from subnormal number should have failed but didn't");
		PortableUnsignedInteger::try_from(f64::INFINITY)
			.expect_err("Creation from infinity should have failed but didn't");
	}
}
