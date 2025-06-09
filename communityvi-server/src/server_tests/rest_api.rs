use crate::reference_time::ReferenceTimer;
use crate::server_tests::start_test_server;
use axum::http::StatusCode;
use js_int::UInt;
use serde::Deserialize;

#[cfg(feature = "api-docs")]
mod api_docs;

#[tokio::test]
async fn should_return_reference_time() {
	let reference_timer = ReferenceTimer::default();
	let client = start_test_server().await;
	let response = client
		.get("/api/reference-time-milliseconds")
		.send()
		.await
		.expect("Request failed");

	let status = response.status();
	let reference_time = response
		.json::<UInt>()
		.await
		.expect("Failed to parse reference time response");

	let reference_time = i64::from(reference_time);
	let expected_reference_time = i64::from(reference_timer.reference_time_milliseconds());
	let diff = (reference_time - expected_reference_time).abs();
	assert_eq!(status, StatusCode::OK);
	assert!(diff >= 0);
	assert!(diff <= 2_000, "Was {diff} ms");
}

#[tokio::test]
async fn should_provide_openapi_json() {
	let client = start_test_server().await;
	let response = client.get("/api/openapi.json").send().await.expect("Request failed");

	// custom struct since rweb::openapi::Spec can't be deserialized from it's own serialization ...
	#[derive(Deserialize)]
	struct OpenApi {
		openapi: String,
	}

	let status = response.status();
	let specification = response
		.json::<OpenApi>()
		.await
		.expect("Failed to deserialize OpenAPI specification from JSON");

	assert_eq!(status, StatusCode::OK);
	assert!(specification.openapi.starts_with("3."));
}
