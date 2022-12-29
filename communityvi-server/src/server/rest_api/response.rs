use aide::gen::GenContext;
use aide::openapi::Operation;
use aide::OperationOutput;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;

/// Response wrapper for HTTP Status Code 201 CREATED
///
/// Just returning a tuple of (`StatusCode`, `Response`) would not be reflected in the aide generated
/// documentation.
pub struct Created<T>(pub T);

impl<T: IntoResponse> IntoResponse for Created<T> {
	fn into_response(self) -> Response {
		(StatusCode::CREATED, self.0).into_response()
	}
}

impl<T: OperationOutput> OperationOutput for Created<T> {
	type Inner = T::Inner;

	fn operation_response(ctx: &mut GenContext, operation: &mut Operation) -> Option<aide::openapi::Response> {
		T::operation_response(ctx, operation)
	}

	fn inferred_responses(
		ctx: &mut GenContext,
		operation: &mut Operation,
	) -> Vec<(Option<u16>, aide::openapi::Response)> {
		T::inferred_responses(ctx, operation)
			.into_iter()
			.map(|(status_code, response)| {
				// Only replace the default 2xx response types, but leave all others intact.
				let status_code = match status_code {
					Some(status_code) if (200..300).contains(&status_code) => Some(StatusCode::CREATED.as_u16()),
					status_code => status_code,
				};
				(status_code, response)
			})
			.collect()
	}
}
