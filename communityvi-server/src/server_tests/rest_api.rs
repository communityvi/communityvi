use crate::server_tests::test_filter;
use js_int::{uint, UInt};
use rweb::http::StatusCode;
use serde::Deserialize;

#[cfg(feature = "api-docs")]
mod api_docs;

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

#[tokio::test]
async fn should_provide_openapi_json() {
	let filter = test_filter();
	let response = rweb::test::request()
		.method("GET")
		.path("/api/openapi.json")
		.reply(&filter)
		.await;

	let status_code = response.status();

	// custom struct since rweb::openapi::Spec can't be deserialized from it's own serialization ...
	#[derive(Deserialize)]
	struct OpenApi {
		openapi: String,
	}
	let specification = serde_json::from_slice::<OpenApi>(response.body())
		.expect("Failed to deserialize OpenAPI specification from JSON");

	assert_eq!(status_code, StatusCode::OK);
	assert!(specification.openapi.starts_with("3."));
}
