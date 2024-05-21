use crate::server::rest_api::error::ApiErrorResponse;
use crate::user::UserCreationError;
use aide::gen::GenContext;
use aide::openapi::{MediaType, Operation, SchemaObject};
use aide::OperationOutput;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use indexmap::IndexMap;
use schemars::JsonSchema;

impl From<UserCreationError> for ApiErrorResponse {
	fn from(error: UserCreationError) -> Self {
		use UserCreationError::*;
		match error {
			NameEmpty => ApiErrorResponse {
				r#type: "user-creation-name-empty",
				status: StatusCode::BAD_REQUEST.as_u16(),
				message: error.to_string(),
			},
			NameTooLong => ApiErrorResponse {
				r#type: "user-creation-name-too-long",
				status: StatusCode::BAD_REQUEST.as_u16(),
				message: error.to_string(),
			},
			NameAlreadyInUse => ApiErrorResponse {
				r#type: "user-creation-name-already-in-use",
				status: StatusCode::BAD_REQUEST.as_u16(),
				message: error.to_string(),
			},
		}
	}
}

impl OperationOutput for UserCreationError {
	type Inner = ApiErrorResponse;

	fn operation_response(ctx: &mut GenContext, _operation: &mut Operation) -> Option<aide::openapi::Response> {
		let schema = SchemaObject {
			json_schema: ApiErrorResponse::json_schema(&mut ctx.schema),
			external_docs: None,
			example: None,
		};

		Some(aide::openapi::Response {
			description: "User could not be created.".to_string(),
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
			vec![(Some(StatusCode::BAD_REQUEST.as_u16()), response)]
		} else {
			Vec::new()
		}
	}
}

impl IntoResponse for UserCreationError {
	fn into_response(self) -> Response {
		ApiErrorResponse::from(self).into_response()
	}
}
