use crate::server::rest_api::error::ApiErrorResponse;
use aide::gen::GenContext;
use aide::openapi::{MediaType, Operation, SchemaObject};
use aide::OperationOutput;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use indexmap::IndexMap;
use schemars::JsonSchema;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub struct AuthenticationFailedError;

impl Display for AuthenticationFailedError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "Authentication failed.")
	}
}

impl Error for AuthenticationFailedError {}

impl From<AuthenticationFailedError> for ApiErrorResponse {
	fn from(_error: AuthenticationFailedError) -> Self {
		ApiErrorResponse {
			r#type: "authentication-failed",
			status: StatusCode::UNAUTHORIZED.as_u16(),
			message: "Authentication failed.".to_string(),
		}
	}
}

impl OperationOutput for AuthenticationFailedError {
	type Inner = ApiErrorResponse;

	fn operation_response(ctx: &mut GenContext, _operation: &mut Operation) -> Option<aide::openapi::Response> {
		let schema = SchemaObject {
			json_schema: ApiErrorResponse::json_schema(&mut ctx.schema),
			external_docs: None,
			example: None,
		};

		Some(aide::openapi::Response {
			description: "Authentication failed.".to_string(),
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
			vec![(Some(StatusCode::UNAUTHORIZED.as_u16()), response)]
		} else {
			Vec::new()
		}
	}
}

impl IntoResponse for AuthenticationFailedError {
	fn into_response(self) -> Response {
		ApiErrorResponse::from(self).into_response()
	}
}
