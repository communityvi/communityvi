use crate::server::rest_api::error::authentication_failed::AuthenticationFailedError;
use crate::user::UserRepository;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::TypedHeader;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
	/// [RFC7519, Section 4.1.2](https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.2)
	sub: String,
}

impl Claims {
	pub fn new(username: String) -> Self {
		Self { sub: username }
	}

	pub fn username(&self) -> &str {
		&self.sub
	}
}

pub async fn middleware<Body>(
	State(user_repository): State<Arc<Mutex<UserRepository>>>,
	State(jwt_decoding_key): State<DecodingKey>,
	TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
	mut request: Request<Body>,
	next: Next<Body>,
) -> Result<Response, AuthenticationFailedError> {
	let mut validation = Validation::new(Algorithm::HS512);
	validation.validate_exp = false;
	validation.required_spec_claims.clear();

	let token = jsonwebtoken::decode::<Claims>(auth_header.token(), &jwt_decoding_key, &validation).map_err(|e| {
		log::debug!("Could not decode token: '{}', error was: {}", auth_header.token(), e);
		AuthenticationFailedError
	})?;

	let Some(user) = user_repository.lock().get(token.claims.username()).cloned() else {
		log::debug!("User for username '{}' not found!", token.claims.username());
		return Err(AuthenticationFailedError);
	};
	request.extensions_mut().insert(user);

	Ok(next.run(request).await)
}

pub fn needs_authentication(operation: TransformOperation) -> TransformOperation {
	operation.response::<401, AuthenticationFailedError>()
}
