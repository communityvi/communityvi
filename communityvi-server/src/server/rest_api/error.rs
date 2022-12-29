use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use schemars::JsonSchema;
use serde::Serialize;

mod user_creation;

/// Type-erased error response
///
/// NOTE: This type is inspired by RFC7807 (Problem Details for HTTP APIs) but spares on a lot of
/// the details to avoid complexity.
///
/// See: <https://www.rfc-editor.org/rfc/rfc7807.html>
#[derive(Serialize, JsonSchema)]
pub struct ApiErrorResponse {
	r#type: &'static str,
	status: u16,
	message: String,
}

impl IntoResponse for ApiErrorResponse {
	fn into_response(self) -> Response {
		let status_code = StatusCode::from_u16(self.status).expect("StatusCode could not be mapped.");
		(status_code, Json(self)).into_response()
	}
}
