// Pass by reference doesn't seem to be supported by rweb here
// NOTE: This is regarding `reference_time_milliseconds` below, but rweb throws these attributes away entirely
//       therefore needs to be global to the module.
#![allow(clippy::needless_pass_by_value)]

use crate::context::ApplicationContext;
use crate::reference_time::ReferenceTimer;
use crate::server::OpenApiJson;
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::transform::TransformOpenApi;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Json, Router};
use tower_http::cors::CorsLayer;

#[cfg(feature = "api-docs")]
mod api_docs;

pub fn rest_api() -> ApiRouter<ApplicationContext> {
	ApiRouter::new()
		.api_route(
			"/reference-time-milliseconds",
			get_with(reference_time_milliseconds,
			|operation| operation
				.summary("Return current server reference time in milliseconds")
				.description("The reference time is the common time that all participants are synchronized on and that all operations refer to.")
			))
		.route("/openapi.json", get(openapi_specification))
		.merge(stoplight_elements())
		.layer(CorsLayer::very_permissive())
}

pub fn finish_openapi_specification(api: TransformOpenApi) -> TransformOpenApi {
	use aide::openapi::Info;
	api.info(Info {
		title: "Communityvi REST API".to_owned(),
		..Default::default()
	})
}

fn stoplight_elements() -> Router<ApplicationContext> {
	#[cfg(not(feature = "api-docs"))]
	{
		Router::new()
	}
	#[cfg(feature = "api-docs")]
	{
		Router::new().nest_service("/docs", axum::routing::get_service(api_docs::api_docs()))
	}
}

async fn openapi_specification(Extension(specification): Extension<OpenApiJson>) -> impl IntoResponse {
	Json(specification)
}

async fn reference_time_milliseconds(State(reference_timer): State<ReferenceTimer>) -> impl IntoApiResponse {
	let milliseconds = u64::from(reference_timer.reference_time_milliseconds());
	Json(milliseconds)
}
