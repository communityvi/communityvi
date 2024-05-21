use crate::user::User;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema)]
pub struct UserRegistrationRequest {
	pub name: String,
}

#[derive(Serialize, JsonSchema)]
pub struct UserRegistrationResponse {
	pub name: String,
}

impl From<User> for UserRegistrationResponse {
	fn from(user: User) -> Self {
		Self {
			name: user.name().to_string(),
		}
	}
}

#[derive(Deserialize, JsonSchema)]
pub struct LoginRequest {
	pub username: String,
}

#[derive(Serialize, JsonSchema)]
pub struct UserResponse {
	pub name: String,
}

impl From<User> for UserResponse {
	fn from(user: User) -> Self {
		UserResponse {
			name: user.name().to_string(),
		}
	}
}
