use crate::error::InternalError;
use hyper::body::Bytes;
use hyper::{http, Body};
use serde::de::DeserializeOwned;
use std::ops::{Deref, DerefMut};

pub struct Response {
	response: http::Response<Body>,
	content: Option<Result<Bytes, crate::Error>>,
}

impl Response {
	pub fn into_body(self) -> Body {
		self.response.into_body()
	}

	pub fn into_response(self) -> http::Response<Body> {
		self.response
	}

	/// NOTE: On the first call, this will take out the body from the response.
	pub async fn content(&mut self) -> Result<Bytes, crate::Error> {
		if self.content.is_none() {
			let mut body = Body::empty();
			std::mem::swap(self.response.body_mut(), &mut body);
			self.content = Some(
				hyper::body::to_bytes(body)
					.await
					.map_err(|error| InternalError::ResponseContent(error.to_string()).into()),
			);
		}

		self.content.as_ref().unwrap_or_else(|| unreachable!()).clone()
	}

	/// NOTE: Calls [`Response::content`]
	pub async fn deserialize_json<T: DeserializeOwned>(&mut self) -> Result<T, crate::Error> {
		let bytes = self.content().await?;
		Ok(serde_json::from_slice(&bytes)?)
	}

	/// NOTE: Calls [`Response::content`]
	pub async fn text(&mut self) -> Result<String, crate::Error> {
		// TODO: Take the encoding header into account instead of always assuming UTF-8
		let bytes = self.content().await?;

		Ok(std::str::from_utf8(&bytes)?.into())
	}
}

impl Deref for Response {
	type Target = http::Response<Body>;

	fn deref(&self) -> &Self::Target {
		&self.response
	}
}

impl DerefMut for Response {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.response
	}
}

impl From<http::Response<Body>> for Response {
	fn from(response: http::Response<Body>) -> Self {
		Self {
			response,
			content: None,
		}
	}
}
