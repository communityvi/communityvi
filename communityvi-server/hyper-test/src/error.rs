use hyper::http;
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;

#[derive(Clone, Debug)]
pub struct Error(InternalError);

#[derive(Clone, Debug)]
pub(crate) enum InternalError {
	Connection(String),
	InvalidHost(String),
	Http(String),
	Json(String),
	TextDecoding(Utf8Error),
	Request(String),
	ResponseContent(String),
}

impl Display for InternalError {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		use InternalError::*;

		match self {
			Connection(message) => write!(formatter, "Connection error: {message}"),
			InvalidHost(message) => write!(formatter, "Invalid host: {message}"),
			Http(message) => write!(formatter, "HTTP error: {message}"),
			Json(message) => write!(formatter, "JSON error: {message}"),
			TextDecoding(error) => write!(formatter, "Text decoding error: {error}"),
			Request(message) => write!(formatter, "Request error: {message}"),
			ResponseContent(message) => write!(formatter, "Failed to read response content: {message}"),
		}
	}
}

impl Display for Error {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		self.0.fmt(formatter)
	}
}

impl std::error::Error for Error {}
impl std::error::Error for InternalError {}

impl From<http::Error> for Error {
	fn from(error: http::Error) -> Self {
		Self(InternalError::Http(error.to_string()))
	}
}

impl From<serde_json::Error> for Error {
	fn from(error: serde_json::Error) -> Self {
		Self(InternalError::Json(error.to_string()))
	}
}

impl From<Utf8Error> for Error {
	fn from(error: Utf8Error) -> Self {
		Self(InternalError::TextDecoding(error))
	}
}

impl From<InternalError> for Error {
	fn from(error: InternalError) -> Self {
		Self(error)
	}
}
