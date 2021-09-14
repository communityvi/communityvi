use crate::server::frontend::etag::ETags;
use gotham::hyper::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use gotham::hyper::{Body, HeaderMap, Response, StatusCode, Uri};
use gotham::state::State;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use mime_guess::MimeGuess;

mod etag;

const FRONTEND_BUILD: Dir = include_dir!("../communityvi-frontend/build");
lazy_static! {
	static ref FRONTEND_ETAGS: ETags = ETags::from(&FRONTEND_BUILD);
}

pub fn frontend_handler(state: State) -> (State, Response<Body>) {
	let uri = state.borrow::<Uri>();

	let path = normalize_path(uri.path());
	let etag = FRONTEND_ETAGS.get(path.as_ref());

	let request_headers = state.borrow::<HeaderMap>();
	if let Some(if_none_match) = request_headers.get(IF_NONE_MATCH) {
		if let Some(etag) = etag {
			if if_none_match == &etag {
				return (state, not_modified());
			}
		}
	}

	match FRONTEND_BUILD.get_file(path) {
		Some(file) => {
			let mime = MimeGuess::from_path(path).first_or_octet_stream();
			let content = file.contents();
			let mut response_builder = Response::builder()
				.status(StatusCode::OK)
				.header(CONTENT_TYPE, mime.as_ref())
				.header(CONTENT_LENGTH, content.len())
				// Tell browsers to always make the request with If-None-Match instead
				// of relying on a maximum age.
				.header(CACHE_CONTROL, "must-revalidate");
			if let Some(etag) = etag {
				response_builder = response_builder.header(ETAG, etag);
			}
			(state, response_builder.body(Body::from(content)).unwrap())
		}
		None => (state, not_found()),
	}
}

fn normalize_path(path: &str) -> &str {
	path.strip_prefix('/').unwrap_or(path)
}

fn not_modified() -> Response<Body> {
	const STATUS: StatusCode = StatusCode::NOT_MODIFIED;
	Response::builder()
		.status(STATUS)
		.body(Body::from(STATUS.canonical_reason().unwrap()))
		.unwrap()
}

fn not_found() -> Response<Body> {
	const STATUS: StatusCode = StatusCode::NOT_FOUND;
	Response::builder()
		.status(STATUS)
		.body(Body::from(STATUS.canonical_reason().unwrap()))
		.unwrap()
}

#[cfg(test)]
mod test {
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
}
