use crate::server_tests::start_test_server;
use rweb::http::StatusCode;

#[tokio::test]
async fn should_serve_overriden_swagger_ui_index_html() {
	let http_client = start_test_server();
	let aliases = ["/api/docs", "/api/docs/"];

	let mut response_to_explicit_path = http_client
		.get("/api/docs/index.html")
		.send()
		.await
		.expect("Request failed.");
	assert_eq!(response_to_explicit_path.status(), StatusCode::OK);

	let response_text = response_to_explicit_path
		.text()
		.await
		.expect("Failed to get response bytes.");
	assert!(response_text.contains("SwaggerUIBundle"));
	// make sure it's not the bundled index file
	assert!(response_text.contains("/api/openapi.json"));

	for alias in aliases {
		let mut response = http_client.get(alias).send().await.expect("Request failed.");
		assert_eq!(
			response.status(),
			response_to_explicit_path.status(),
			"Status for alias '{alias}' was different from the explicit path.",
		);
		assert_eq!(
			response.content().await.expect("Failed to get response bytes."),
			response_to_explicit_path
				.content()
				.await
				.expect("Failed to get response bytes."),
			"Response for alias '{alias}' was different from the explicit path.",
		);
	}
}

#[tokio::test]
async fn should_serve_bundled_swagger_ui() {
	let http_client = start_test_server();

	// sample some of the files
	for filename in ["index.html", "swagger-ui.js", "LICENSE"] {
		let mut response = http_client
			.get(&format!("/api/docs/{filename}"))
			.send()
			.await
			.expect("Request failed.");

		assert_eq!(response.status(), StatusCode::OK, "Missing file '{filename}'");
		let content = response.content().await.expect("Failed to get response bytes.");
		assert!(!content.is_empty(), "File '{filename}' is empty.");
	}
}
