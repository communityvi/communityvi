use crate::server_tests::test_filter;
use js_int::{uint, UInt};
use rweb::http::StatusCode;

#[tokio::test]
async fn should_return_reference_time() {
	let filter = test_filter();
	let response = rweb::test::request()
		.method("GET")
		.path("/api/reference_time_milliseconds")
		.reply(&filter)
		.await;

	let status_code = response.status();
	let content = serde_json::from_slice::<UInt>(response.body()).expect("Failed to parse reference time JSON");

	assert_eq!(status_code, StatusCode::OK);
	assert!(content >= uint!(0));
	assert!(content <= uint!(1_000));
}
