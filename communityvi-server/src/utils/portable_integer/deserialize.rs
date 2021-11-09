use crate::utils::portable_integer::PortableInteger;
use serde::de::{Unexpected, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt::Formatter;

impl<'de> Deserialize<'de> for PortableInteger {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_any(PortableIntegerVisitor)
	}
}

struct PortableIntegerVisitor;

impl PortableIntegerVisitor {
	const EXPECTING: &'static str = "an integer or integral float from 0 to 2^53 (inclusive)";
}

impl Visitor<'_> for PortableIntegerVisitor {
	type Value = PortableInteger;

	fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
		formatter.write_str(Self::EXPECTING)
	}

	fn visit_i64<E>(self, number: i64) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		PortableInteger::try_from(number).map_err(|_| E::invalid_value(Unexpected::Signed(number), &Self::EXPECTING))
	}

	fn visit_u64<E>(self, number: u64) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		PortableInteger::try_from(number).map_err(|_| E::invalid_value(Unexpected::Unsigned(number), &Self::EXPECTING))
	}

	fn visit_f64<E>(self, number: f64) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		PortableInteger::try_from(number).map_err(|_| E::invalid_value(Unexpected::Float(number), &Self::EXPECTING))
	}
}

#[cfg(test)]
mod test {
	use crate::utils::portable_integer::PortableInteger;

	#[test]
	fn portable_integer_can_be_deserialized_from_integer() {
		let number = serde_json::from_str::<PortableInteger>("42").expect("Failed to deserialize from integer JSON.");
		assert_eq!(PortableInteger::from(42u32), number);
	}

	#[test]
	fn portable_integer_can_be_deserialized_from_float() {
		let number = serde_json::from_str::<PortableInteger>("42.0").expect("Failed to deserialize from float JSON.");
		assert_eq!(PortableInteger::from(42u32), number);
	}

	#[test]
	fn portable_integer_cannot_be_deserialized_from_invalid_number() {
		serde_json::from_str::<PortableInteger>(&u64::MAX.to_string())
			.expect_err("Deserialization from u64::MAX should have failed, but didn't");
		serde_json::from_str::<PortableInteger>("-1")
			.expect_err("Deserialization from negative number should have failed, but didn't");
		serde_json::from_str::<PortableInteger>("0.5")
			.expect_err("Deserialization from fractional number should have failed, but didn't");
		serde_json::from_str::<PortableInteger>("-1.0")
			.expect_err("Deserialization from negative number should have failed, but didn't");
	}
}
