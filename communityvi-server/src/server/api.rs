use crate::server::session::SessionStore;
use crate::user::User;
use rweb::filters::BoxedFilter;
use rweb::openapi::{Server, Spec};
use rweb::{any, get, openapi, router, warp, Filter, Query, Reply, Schema};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

mod sessions;

pub type Sessions = SessionStore<String>;

pub fn rest_api() -> BoxedFilter<(impl Reply,)> {
	const EXPIRE_SESSION_AFTER: Duration = Duration::from_secs(24 * 3600);
	const MAXIMUM_SESSION_COUNT: usize = 100;

	let user_store = Sessions::new(EXPIRE_SESSION_AFTER, MAXIMUM_SESSION_COUNT);
	let (mut spec, filter) = openapi::spec().build(move || sessions::sessions(user_store));
	// Required to make it work with the API at a different path than `/`
	spec.servers.push(Server {
		url: "./".into(),
		description: Default::default(),
		variables: Default::default(),
	});

	filter.or(openapi_docs(spec)).boxed()
}

// Based on https://github.com/kdy1/rweb/blob/3f4001dd52215c12f22f369acd5863bda2ae7364/src/docs.rs
// The change is that '/openapi.json` was replaced with './openapi.json' to allow the api being
// served under a different path than `/`.
pub fn openapi_docs(spec: Spec) -> BoxedFilter<(impl Reply,)> {
	let docs_openapi = warp::path("openapi.json").map(move || warp::reply::json(&spec.to_owned()));
	let docs = warp::path("docs").map(|| {
		warp::reply::html(
			r#"
			<!doctype html>
			<html lang="en">
			<head>
			<title>rweb</title>
			<link href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@3/swagger-ui.css" rel="stylesheet">
			</head>
			<body>
				<div id="swagger-ui"></div>
				<script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@3/swagger-ui-bundle.js" charset="UTF-8"> </script>
				<script>
					window.onload = function() {
					const ui = SwaggerUIBundle({
						"dom_id": "\#swagger-ui",
						presets: [
						SwaggerUIBundle.presets.apis,
						SwaggerUIBundle.SwaggerUIStandalonePreset
						],
						layout: "BaseLayout",
						deepLinking: true,
						showExtensions: true,
						showCommonExtensions: true,
						url: "./openapi.json",
					})
					window.ui = ui;
				};
			</script>
			</body>
			</html>
	"#,
		)
	});
	docs.or(docs_openapi).boxed()
}
