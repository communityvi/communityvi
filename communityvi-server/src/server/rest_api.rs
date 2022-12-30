// Pass by reference doesn't seem to be supported by rweb here
// NOTE: This is regarding `reference_time_milliseconds` below, but rweb throws these attributes away entirely
//       therefore needs to be global to the module.
#![allow(clippy::needless_pass_by_value)]

use crate::context::ApplicationContext;
use crate::reference_time::ReferenceTimer;
use crate::server::rest_api::error::login::LoginError;
use crate::server::rest_api::models::{LoginRequest, UserRegistrationRequest, UserRegistrationResponse};
use crate::server::rest_api::response::Created;
use crate::server::OpenApiJson;
use crate::user::{UserCreationError, UserRepository};
use aide::axum::routing::{get_with, post_with};
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::transform::TransformOpenApi;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Json, Router};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use parking_lot::Mutex;
use std::sync::Arc;

#[cfg(feature = "api-docs")]
mod api_docs;
mod error;
mod models;
mod response;

pub fn rest_api() -> ApiRouter<ApplicationContext> {
	ApiRouter::new()
		.api_route(
			"/reference-time-milliseconds",
			get_with(reference_time_milliseconds,
			|operation| operation
				.summary("Return current server reference time in milliseconds")
				.description("The reference time is the common time that all participants are synchronized on and that all operations refer to.")
			))
		.api_route("/user", post_with(register_user, |operation| operation
			.summary("Register a user")
			.description("Users need to be registered before they can take part in any room.")
		))
		.api_route("/login", post_with(login, |operation| operation
			.summary("Perform a login")
			.description("Creates a JWT on success that can be used to authenticate as the user.")
		))
		.route("/openapi.json", get(openapi_specification))
		.merge(stoplight_elements())
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

async fn register_user(
	State(user_repository): State<Arc<Mutex<UserRepository>>>,
	Json(request): Json<UserRegistrationRequest>,
) -> Result<impl IntoApiResponse, UserCreationError> {
	let user = user_repository.lock().create_user(&request.name)?;
	Ok(Created(Json(UserRegistrationResponse::from(user))))
}

async fn login(
	State(user_repository): State<Arc<Mutex<UserRepository>>>,
	State(jwt_encoding_key): State<EncodingKey>,
	Json(request): Json<LoginRequest>,
) -> Result<impl IntoApiResponse, LoginError> {
	let Some(user) = user_repository.lock().get(&request.username).cloned() else {
		return Err(LoginError::UnknownUser);
	};

	// FIXME: Rework once infrastructure is in place.
	let header = Header::new(Algorithm::HS512);
	let key = jsonwebtoken::encode(&header, &(user.name().to_string()), &jwt_encoding_key)?;

	Ok(Json(key))
}
