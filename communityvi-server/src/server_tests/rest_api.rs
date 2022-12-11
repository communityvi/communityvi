use crate::server_tests::start_test_server;
use axum::http::StatusCode;
use js_int::{uint, UInt};
use serde::Deserialize;

#[cfg(feature = "api-docs")]
mod api_docs;

#[tokio::test]
async fn should_return_reference_time() {
	let client = start_test_server();
	let mut response = client
		.get("/api/reference-time-milliseconds")
		.send()
		.await
		.expect("Request failed");

	let reference_time = response
		.deserialize_json::<UInt>()
		.await
		.expect("Failed to parse reference time response");

	assert_eq!(response.status(), StatusCode::OK);
	assert!(reference_time >= uint!(0));
	assert!(reference_time <= uint!(2_000), "Was {reference_time} ms");
}

#[tokio::test]
async fn should_provide_openapi_json() {
	let client = start_test_server();
	let mut response = client.get("/api/openapi.json").send().await.expect("Request failed");

	// custom struct since rweb::openapi::Spec can't be deserialized from it's own serialization ...
	#[derive(Deserialize)]
	struct OpenApi {
		openapi: String,
	}

	let specification = response
		.deserialize_json::<OpenApi>()
		.await
		.expect("Failed to deserialize OpenAPI specification from JSON");

	assert_eq!(response.status(), StatusCode::OK);
	assert!(specification.openapi.starts_with("3."));
}
