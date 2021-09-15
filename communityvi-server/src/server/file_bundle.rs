use crate::server::etag::ETags;
use gotham::handler::{Handler, HandlerFuture};
use gotham::hyper::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use gotham::hyper::{Body, HeaderMap, Response, StatusCode, Uri};
use gotham::state::State;
use include_dir::{Dir, File};
use mime_guess::MimeGuess;
use std::pin::Pin;

pub struct BundledFileHandler {
	directory: Dir<'static>,
	etags: ETags,
}

impl From<Dir<'static>> for BundledFileHandler {
	fn from(directory: Dir<'static>) -> Self {
		Self {
			directory,
			etags: ETags::from(&directory),
		}
	}
}

impl Handler for BundledFileHandler {
	fn handle(self, state: State) -> Pin<Box<HandlerFuture>> {
		let uri = state.borrow::<Uri>();
		let request_headers = state.borrow::<HeaderMap>();
		let response = self.handle_request(uri.path(), request_headers);

		Box::pin(std::future::ready(Ok((state, response))))
	}
}

impl BundledFileHandler {
	fn handle_request(&self, path: &str, request_headers: &HeaderMap) -> Response<Body> {
		let file = match self.get_file_falling_back_to_index_html(path) {
			Some(file) => file,
			None => return not_found(),
		};
		let path = file.path();

		let mut response_builder = Response::builder();
		if let Some(etag) = self.etags.get(path.as_ref()) {
			response_builder = response_builder.header(ETAG, etag);

			if let Some(if_none_match) = request_headers.get(IF_NONE_MATCH) {
				if if_none_match == &etag {
					return not_modified();
				}
			}
		}

		let mime = MimeGuess::from_path(file.path()).first_or_octet_stream();
		let content = file.contents();
		response_builder
			.status(StatusCode::OK)
			.header(CONTENT_TYPE, mime.as_ref())
			.header(CONTENT_LENGTH, content.len())
			// Tell browsers to always make the request with If-None-Match instead
			// of relying on a maximum age.
			.header(CACHE_CONTROL, "must-revalidate")
			.body(Body::from(content))
			.unwrap()
	}

	fn get_file_falling_back_to_index_html(&self, path: &str) -> Option<File<'static>> {
		match self.get_file(path) {
			Some(file) => Some(file),
			None => self.get_file(&format!("{}/index.html", path)),
		}
	}

	fn get_file(&self, path: &str) -> Option<File<'static>> {
		self.directory.get_file(Self::normalize_path(path))
	}

	fn normalize_path(path: &str) -> &str {
		path.trim_matches('/')
	}
}

fn not_found() -> Response<Body> {
	const STATUS: StatusCode = StatusCode::NOT_FOUND;
	Response::builder()
		.status(STATUS)
		.header(CACHE_CONTROL, "no-cache")
		.body(Body::from(STATUS.canonical_reason().unwrap()))
		.unwrap()
}

fn not_modified() -> Response<Body> {
	const STATUS: StatusCode = StatusCode::NOT_MODIFIED;
	Response::builder()
		.status(STATUS)
		.body(Body::from(STATUS.canonical_reason().unwrap()))
		.unwrap()
}

#[cfg(test)]
mod test {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn path_equality_learning_tests() {
		let no_trailing_slash = PathBuf::from("a/b/c");
		let trailing_slash = PathBuf::from("a/b/c/");
		let double_slash = PathBuf::from("a//b/c/");
		let dot_dot = PathBuf::from("a/b/../b/c");

		assert_eq!(no_trailing_slash.as_path(), trailing_slash.as_path());
		assert_eq!(trailing_slash.as_path(), double_slash.as_path());
		assert_eq!(no_trailing_slash.as_path(), double_slash.as_path());
		assert_ne!(no_trailing_slash.as_path(), dot_dot.as_path());
	}

	#[test]
	fn normalize_path_should_strip_slashes() {
		assert_eq!("", BundledFileHandler::normalize_path("/"));
		assert_eq!("index.html", BundledFileHandler::normalize_path("index.html/"));
		assert_eq!("index.html", BundledFileHandler::normalize_path("/index.html"));
		assert_eq!("index.html", BundledFileHandler::normalize_path("/index.html/"));
	}
}
