use serde::Serialize;
use std::error::Error;
use std::fmt::{Display, Formatter};

mod deserialize;

/// Integer that works across modern programming languages, even those that only
/// support IEE754 Double Precision Floating Point numbers like JavaScript.
#[derive(Serialize, PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct PortableInteger(u64);

impl PortableInteger {
	pub const MAX: u64 = (2u64.pow(53) - 1);
	pub const MIN: u64 = 0;
}

impl From<u32> for PortableInteger {
	fn from(number: u32) -> Self {
		Self(number.into())
	}
}

impl From<u16> for PortableInteger {
	fn from(number: u16) -> Self {
		Self(number.into())
	}
}

impl From<u8> for PortableInteger {
	fn from(number: u8) -> Self {
		Self(number.into())
	}
}

impl TryFrom<u64> for PortableInteger {
	type Error = InvalidNumber;

	fn try_from(number: u64) -> Result<Self, Self::Error> {
		if !(PortableInteger::MIN..PortableInteger::MAX).contains(&number) {
			return Err(InvalidNumber::new(number));
		}

		Ok(PortableInteger(number))
	}
}

impl TryFrom<i64> for PortableInteger {
	type Error = InvalidNumber;

	fn try_from(signed: i64) -> Result<Self, Self::Error> {
		let unsigned = u64::try_from(signed).map_err(|_| InvalidNumber::new(signed))?;
		Self::try_from(unsigned)
	}
}

impl TryFrom<f64> for PortableInteger {
	type Error = InvalidNumber;

	fn try_from(number: f64) -> Result<Self, Self::Error> {
		if !number.is_normal() {
			return Err(InvalidNumber::new(number));
		}

		if number.fract() != 0.0 {
			return Err(InvalidNumber::new(number));
		}

		// truncation is explicit and assured to work since
		// we checked that the double value is "normal", meaning
		// neither NaN, infinite or "subnormal" (which are never integers)
		#[allow(clippy::cast_possible_truncation)]
		let integer = number.trunc() as i64;
		Self::try_from(integer)
	}
}

impl From<PortableInteger> for u64 {
	fn from(PortableInteger(number): PortableInteger) -> Self {
		number
	}
}

impl Display for PortableInteger {
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
	use crate::utils::portable_integer::PortableInteger;

	#[test]
	fn portable_integer_can_be_serialized() {
		let number = PortableInteger(42);

		let json = serde_json::to_string(&number).expect("Failed to serialize number.");
		assert_eq!("42", json);
	}

	#[test]
	fn portable_integer_can_be_created_from_u64() {
		let number = PortableInteger::try_from(42u64).expect("Failed to create from u64");
		assert_eq!(PortableInteger(42), number);
	}

	#[test]
	fn portable_integer_cannot_be_created_from_u64_out_of_range() {
		PortableInteger::try_from(u64::MAX).expect_err("Creation from u64::MAX should have failed but didn't");
	}

	#[test]
	fn portable_integer_cannot_be_created_from_i64_out_of_range() {
		PortableInteger::try_from(i64::MAX).expect_err("Creation from i64::MAX should have failed but didn't");
		PortableInteger::try_from(-1i64).expect_err("Creation from negative number should have failed but didn't");
	}

	#[test]
	fn portable_integer_can_be_created_from_f64() {
		let number = PortableInteger::try_from(42.0).expect("Failed to create from f64");
		assert_eq!(PortableInteger(42), number);
	}

	#[test]
	fn portable_integer_cannot_be_created_from_incompatible_f64() {
		PortableInteger::try_from(-1.0).expect_err("Creation from negative number should have failed but didn't");
		PortableInteger::try_from(0.5).expect_err("Creation from fractional number should have failed but didn't");
		PortableInteger::try_from(f64::MAX).expect_err("Creation from f64::MAX should have failed but didn't");
		PortableInteger::try_from(f64::NAN).expect_err("Creation from NaN should have failed but didn't");
		const SUBNORMAL_NUMBER: f64 = 1.0e-308_f64;
		assert!(SUBNORMAL_NUMBER.is_subnormal());
		PortableInteger::try_from(SUBNORMAL_NUMBER)
			.expect_err("Creation from subnormal number should have failed but didn't");
		PortableInteger::try_from(f64::INFINITY).expect_err("Creation from infinity should have failed but didn't");
	}
}
