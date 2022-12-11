use crate::server_tests::start_test_server;
use axum::http::StatusCode;

#[tokio::test]
async fn should_serve_bundled_stoplight_elements() {
	let client = start_test_server();

	// sample some of the files
	for filename in ["index.html", "web-components.min.js", "LICENSE"] {
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
