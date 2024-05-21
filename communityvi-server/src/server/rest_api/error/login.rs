use crate::server::rest_api::error::ApiErrorResponse;
use aide::gen::GenContext;
use aide::openapi::{MediaType, Operation, SchemaObject};
use aide::OperationOutput;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use indexmap::IndexMap;
use schemars::JsonSchema;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum LoginError {
	#[error("Invalid credentials.")]
	UnknownUser,
	#[error("Login failed.")]
	JwtEncodeError(#[from] jsonwebtoken::errors::Error),
}

impl From<LoginError> for ApiErrorResponse {
	fn from(error: LoginError) -> Self {
		use LoginError::*;
		match error {
			UnknownUser => ApiErrorResponse {
				r#type: "login-access-denied",
				status: StatusCode::UNAUTHORIZED.as_u16(),
				message: "Login was rejected.".to_string(),
			},
			JwtEncodeError(_) => ApiErrorResponse {
				r#type: "login-internal-error",
				status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
				message: "Login failed.".to_string(),
			},
		}
	}
}

impl OperationOutput for LoginError {
	type Inner = ApiErrorResponse;

	fn operation_response(ctx: &mut GenContext, _operation: &mut Operation) -> Option<aide::openapi::Response> {
		let schema = SchemaObject {
			json_schema: ApiErrorResponse::json_schema(&mut ctx.schema),
			external_docs: None,
			example: None,
		};

		Some(aide::openapi::Response {
			description: "User could not be logged in.".to_string(),
			content: IndexMap::from_iter([(
				mime::APPLICATION_JSON.to_string(),
				MediaType {
					schema: Some(schema),
					..Default::default()
				},
			)]),
			..Default::default()
		})
	}

	fn inferred_responses(
		ctx: &mut GenContext,
		operation: &mut Operation,
	) -> Vec<(Option<u16>, aide::openapi::Response)> {
		if let Some(response) = Self::operation_response(ctx, operation) {
			vec![
				(Some(StatusCode::UNAUTHORIZED.as_u16()), response.clone()),
				(Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()), response),
			]
		} else {
			Vec::new()
		}
	}
}

impl IntoResponse for LoginError {
	fn into_response(self) -> Response {
		ApiErrorResponse::from(self).into_response()
	}
}
