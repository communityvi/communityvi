// Pass by reference doesn't seem to be supported by rweb here
// NOTE: This is regarding `reference_time_milliseconds` below, but rweb throws these attributes away entirely
//       therefore needs to be global to the module.
#![allow(clippy::needless_pass_by_value)]
use crate::reference_time::ReferenceTimer;
use axum::extract::Extension;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use okapi::openapi3::{self, Info, OpenApi, Operation, PathItem, Responses};

#[cfg(feature = "api-docs")]
mod api_docs;

pub fn rest_api(reference_timer: ReferenceTimer) -> Router {
	let specification = OpenApi {
		openapi: "3.0.1".into(),
		info: Info {
			title: "Communityvi REST API".into(),
			..Default::default()
		},
		servers: vec![],
		paths: [(
			"/api/reference-time-milliseconds",
			PathItem {
				get: Some(Operation {
					summary: Some("Returns the current server reference time in milliseconds.".into()),
					description: Some("The reference time is the common time that all participants are synchronized on and that all operations refer to.".into()),
					responses: Responses::default(),
					..Default::default()
				}),
				..Default::default()
			},
		)].into_iter().map(|(path, item)| (path.to_string(), item)).collect(),
		components: None,
		security: vec![],
		tags: vec![],
		external_docs: None,
		extensions: Default::default(),
	};

	Router::new()
		.route("/reference-time-milliseconds", get(reference_time_milliseconds))
		.route_layer(Extension(reference_timer))
		.merge(openapi_router(specification))
}

fn openapi_router(specification: openapi3::OpenApi) -> Router {
	let spec_json = move || async move { axum::response::Json(specification.clone()) };

	#[cfg(not(feature = "api-docs"))]
	{
		Router::new().route("/openapi.json", get(spec_json))
	}
	#[cfg(feature = "api-docs")]
	{
		Router::new()
			.route("/openapi.json", get(spec_json))
			.nest_service("/docs", axum::routing::get_service(api_docs::api_docs()))
	}
}

async fn reference_time_milliseconds(Extension(reference_timer): Extension<ReferenceTimer>) -> impl IntoResponse {
	let milliseconds = u64::from(reference_timer.reference_time_milliseconds());
	axum::response::Json(milliseconds)
}
