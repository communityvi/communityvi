use crate::server_tests::start_test_server;
use axum::http::StatusCode;
use hyper_test::hyper;

#[tokio::test]
async fn should_serve_overriden_swagger_ui_index_html() {
	let client = start_test_server();

	let aliases = [
		//"/api/docs", // redirects to /api/docs/
		"/api/docs/",
	];

	let response_to_explicit_path = client.get("/api/docs/index.html").send().await.expect("Request failed");
	assert_eq!(response_to_explicit_path.status(), StatusCode::OK);
	let explicit_status = response_to_explicit_path.status();
	let explicit_content = hyper::body::to_bytes(response_to_explicit_path.into_body())
		.await
		.expect("Failed to read bytes");
	let response_string = String::from_utf8_lossy(&explicit_content);
	assert!(response_string.contains("SwaggerUIBundle"));
	// make sure it's not the bundled index file
	assert!(response_string.contains("/api/openapi.json"));

	for alias in aliases {
		let response = client.get(alias).send().await.expect("Request failed");
		assert_eq!(
			response.status(),
			explicit_status,
			"Status for alias '{}' was different from the explicit path.",
			alias
		);
		assert_eq!(
			hyper::body::to_bytes(response.into_body())
				.await
				.expect("Failed to read bytes"),
			explicit_content,
			"Response for alias '{}' was different from the explicit path.",
			alias
		);
	}
}

#[tokio::test]
async fn should_serve_bundled_swagger_ui() {
	let client = start_test_server();

	// sample some of the files
	for filename in ["index.html", "swagger-ui.js", "LICENSE"] {
		let mut response = client
			.get(&format!("/api/docs/{filename}"))
			.send()
			.await
			.expect("Request failed");

		assert_eq!(response.status(), StatusCode::OK, "Missing file '{filename}'");
		let content = response.content().await.expect("Failed to get response bytes.");
		assert!(!content.is_empty(), "File '{filename}' is empty.");
	}
}
