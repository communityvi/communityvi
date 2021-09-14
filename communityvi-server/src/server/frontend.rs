use crate::server::frontend::etag::ETags;
use gotham::hyper::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use gotham::hyper::{Body, HeaderMap, Response, StatusCode, Uri};
use gotham::state::State;
use include_dir::{include_dir, Dir, File};
use lazy_static::lazy_static;
use mime_guess::MimeGuess;

mod etag;

const FRONTEND_BUILD: Dir = include_dir!("../communityvi-frontend/build");
lazy_static! {
	static ref FRONTEND_ETAGS: ETags = ETags::from(&FRONTEND_BUILD);
}

pub fn frontend_handler(state: State) -> (State, Response<Body>) {
	let uri = state.borrow::<Uri>();
	let file = match get_bundled_file_falling_back_to_index_html(uri.path()) {
		Some(file) => file,
		None => return (state, not_found()),
	};

	let mut response_builder = Response::builder();
	if let Some(etag) = FRONTEND_ETAGS.get(file.path().as_ref()) {
		response_builder = response_builder.header(ETAG, etag);

		let request_headers = state.borrow::<HeaderMap>();
		if let Some(if_none_match) = request_headers.get(IF_NONE_MATCH) {
			if if_none_match == &etag {
				return (state, not_modified());
			}
		}
	}

	let mime = MimeGuess::from_path(file.path()).first_or_octet_stream();
	let content = file.contents();
	response_builder = response_builder
		.status(StatusCode::OK)
		.header(CONTENT_TYPE, mime.as_ref())
		.header(CONTENT_LENGTH, content.len())
		// Tell browsers to always make the request with If-None-Match instead
		// of relying on a maximum age.
		.header(CACHE_CONTROL, "must-revalidate");

	(state, response_builder.body(Body::from(content)).unwrap())
}

fn get_bundled_file_falling_back_to_index_html(path: &str) -> Option<File<'static>> {
	match get_bundled_file(path) {
		Some(file) => Some(file),
		None => get_bundled_file(&format!("{}/index.html", path)),
	}
}

fn get_bundled_file(path: &str) -> Option<File<'static>> {
	FRONTEND_BUILD.get_file(normalize_path(path))
}

fn normalize_path(path: &str) -> &str {
	path.trim_matches('/')
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
