use crate::server_tests::test_filter;
use rweb::http::StatusCode;

#[tokio::test]
async fn should_display_overriden_swagger_ui_index_html() {
	let filter = test_filter();
	let aliases = ["/api/docs", "/api/docs/"];

	let response_to_explicit_path = rweb::test::request()
		.method("GET")
		.path("/api/docs/index.html")
		.reply(&filter)
		.await;
	assert_eq!(response_to_explicit_path.status(), StatusCode::OK);
	let response_string = String::from_utf8_lossy(response_to_explicit_path.body());
	assert!(response_string.contains("SwaggerUIBundle"));
	// make sure it's not the bundled index file
	assert!(response_string.contains("/api/openapi.json"));

	for alias in aliases {
		let response = rweb::test::request().method("GET").path(alias).reply(&filter).await;
		assert_eq!(
			response.status(),
			response_to_explicit_path.status(),
			"Status for alias '{}' was different from the explicit path.",
			alias
		);
		assert_eq!(
			response.body(),
			response_to_explicit_path.body(),
			"Response for alias '{}' was different from the explicit path.",
			alias
		);
	}
}
