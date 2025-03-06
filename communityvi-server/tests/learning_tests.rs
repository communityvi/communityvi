use js_int::{Int, UInt, int, uint};

#[test]
fn js_int_supports_deserializing_from_floats() {
	let positive_json = "42.0";
	let negative_json = "-42.0";

	let uint = serde_json::from_str::<UInt>(positive_json).unwrap();
	let int = serde_json::from_str::<Int>(negative_json).unwrap();

	assert_eq!(uint, uint!(42));
	assert_eq!(int, int!(-42));
}

#[test]
fn js_int_declines_deserializing_from_fractional_float() {
	let positive_json = "42.5";
	let negative_json = "-42.5";

	serde_json::from_str::<UInt>(positive_json).expect_err("Deserialized UInt although it shouldn't");
	serde_json::from_str::<Int>(negative_json).expect_err("Deserialized Int although it shouldn't");
}
