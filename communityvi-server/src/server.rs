#![allow(clippy::unused_async)]
use crate::context::ApplicationContext;
use crate::room::Room;
use crate::server::rest_api::{finish_openapi_specification, rest_api};
use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use axum::extract::Extension;
use axum::Router;
use serde::{Serialize, Serializer};
use serde_json::value::RawValue;
use std::sync::Arc;

mod file_bundle;
mod rest_api;

pub async fn run_server(application_context: ApplicationContext) {
	let room = Room::new(
		application_context.reference_timer.clone(),
		application_context.configuration.room_size_limit,
	);
	let address = application_context.configuration.address;
	axum::Server::bind(&address)
		.serve(create_router(application_context, room).into_make_service())
		.await
		.unwrap();
}

pub fn create_router(application_context: ApplicationContext, room: Room) -> Router {
	let mut api_specification = OpenApi::default();

	aide::gen::infer_responses(true);
	aide::gen::extract_schemas(true);
	aide::gen::all_error_responses(true);

	let router = ApiRouter::new()
		.nest_api_service("/api", rest_api(application_context.clone()))
		.finish_api_with(&mut api_specification, finish_openapi_specification)
		.with_state(application_context)
		.layer(Extension(room))
		.layer(Extension(
			OpenApiJson::try_from(api_specification).expect("Failed to serialize generated OpenAPI specification"),
		));

	#[cfg(feature = "bundle-frontend")]
	{
		#[derive(rust_embed::RustEmbed)]
		#[folder = "$CARGO_MANIFEST_DIR/../communityvi-frontend/build"]
		struct FrontendBundle;

		let bundled_frontend_handler = file_bundle::BundledFileHandlerBuilder::default()
			.with_rust_embed::<FrontendBundle>()
			.build();
		router.fallback_service(axum::routing::get_service(bundled_frontend_handler))
	}

	#[cfg(not(feature = "bundle-frontend"))]
	{
		router
	}
}

#[derive(Clone)]
struct OpenApiJson(Arc<RawValue>);

impl TryFrom<OpenApi> for OpenApiJson {
	type Error = serde_json::Error;

	fn try_from(value: OpenApi) -> Result<Self, Self::Error> {
		let json = serde_json::to_string(&value)?;
		serde_json::from_str::<Box<RawValue>>(&json).map(|raw_json| Self(raw_json.into()))
	}
}

impl Serialize for OpenApiJson {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		RawValue::serialize(&self.0, serializer)
	}
}
