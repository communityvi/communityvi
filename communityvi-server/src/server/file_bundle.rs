use crate::server::etag::{ETag, ETags};
use include_dir::{Dir, File};
use mime_guess::MimeGuess;
use rweb::hyper::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use rweb::hyper::{Body, HeaderMap, Response, StatusCode};

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

impl BundledFileHandler {
	pub fn handle_request(&self, path: &str, request_headers: &HeaderMap) -> Response<Body> {
		let (file, etag) = match self.get_file_and_etag_falling_back_to_index_html(path) {
			Some(file_and_etag) => file_and_etag,
			None => return not_found(),
		};

		if Self::file_is_cached(etag, request_headers) {
			return not_modified();
		}

		ok(file, etag)
	}

	fn get_file_and_etag_falling_back_to_index_html(&self, path: &str) -> Option<(File<'static>, ETag)> {
		match self.get_file_and_etag(path) {
			Some(file_and_etag) => Some(file_and_etag),
			None => self.get_file_and_etag(&format!("{}/index.html", path)),
		}
	}

	fn get_file_and_etag(&self, path: &str) -> Option<(File<'static>, ETag)> {
		let path = Self::normalize_path(path);
		self.directory.get_file(Self::normalize_path(path)).map(|file| {
			let etag = self
				.etags
				.get(path.as_ref())
				.expect("Found file without ETag, this mustn't happen!");
			(file, etag)
		})
	}

	fn file_is_cached(etag: ETag, request_headers: &HeaderMap) -> bool {
		match request_headers.get(IF_NONE_MATCH) {
			Some(if_none_match) => if_none_match == &etag,
			None => false,
		}
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

fn ok(file: File<'static>, etag: ETag) -> Response<Body> {
	let mime = MimeGuess::from_path(file.path()).first_or_octet_stream();
	let content = file.contents();
	Response::builder()
		.status(StatusCode::OK)
		.header(CONTENT_TYPE, mime.as_ref())
		.header(CONTENT_LENGTH, content.len())
		// Tell browsers to always make the request with If-None-Match instead
		// of relying on a maximum age.
		.header(CACHE_CONTROL, "must-revalidate")
		.header(ETAG, etag)
		.body(Body::from(content))
		.unwrap()
}

#[cfg(test)]
mod test {
	use super::*;
	use include_dir::include_dir;
	use rweb::http::HeaderValue;
	use rweb::hyper::body::Bytes;

	const BUNDLED_TEST_FILES: Dir = include_dir!("test/bundled_files");

	#[tokio::test]
	async fn request_handler_should_return_files() {
		let handler = test_handler();
		let index = BUNDLED_TEST_FILES.get_file("index.html").unwrap();

		let response = handler.handle_request("index.html", &HeaderMap::default());
		let status_code = response.status();
		let content = content(response).await;

		assert_eq!(index.contents(), content);
		assert_eq!(StatusCode::OK, status_code);
	}

	#[test]
	fn request_handler_should_reply_with_not_found_if_file_is_not_found() {
		let handler = test_handler();

		let response = handler.handle_request("nonexistent", &HeaderMap::default());

		assert_eq!(StatusCode::NOT_FOUND, response.status());
	}

	#[test]
	fn request_handler_should_reply_with_not_modified_if_etag_matches() {
		const PATH: &str = "about/index.html";
		let handler = test_handler();
		let uncached_response = handler.handle_request(PATH, &HeaderMap::default());
		let etag = uncached_response.headers()[ETAG].as_bytes();

		let mut request_headers = HeaderMap::new();
		request_headers.insert(IF_NONE_MATCH, HeaderValue::from_bytes(etag).unwrap());

		let cached_response = handler.handle_request(PATH, &request_headers);

		assert_eq!(StatusCode::NOT_MODIFIED, cached_response.status());
	}

	#[test]
	fn request_handler_should_return_file_with_not_modified_if_etag_does_not_match() {
		const PATH: &str = "about/index.html";
		let handler = test_handler();

		let mut request_headers = HeaderMap::new();
		request_headers.insert(IF_NONE_MATCH, "wrong_etag".parse().unwrap());

		let response = handler.handle_request(PATH, &request_headers);

		assert_eq!(StatusCode::OK, response.status());
	}

	fn test_handler() -> BundledFileHandler {
		BundledFileHandler::from(BUNDLED_TEST_FILES)
	}

	#[test]
	fn normalize_path_should_strip_slashes() {
		assert_eq!("", BundledFileHandler::normalize_path("/"));
		assert_eq!("index.html", BundledFileHandler::normalize_path("index.html/"));
		assert_eq!("index.html", BundledFileHandler::normalize_path("/index.html"));
		assert_eq!("index.html", BundledFileHandler::normalize_path("/index.html/"));
	}

	#[test]
	fn ok_responses_should_contain_the_expected_cache_control_header() {
		let file = BUNDLED_TEST_FILES.get_file("index.html").unwrap();
		let etag = ETag::from(&file);

		let response = ok(file, etag);
		let headers = response.headers();

		assert_eq!("must-revalidate", headers[CACHE_CONTROL])
	}

	#[test]
	fn ok_responses_should_contain_the_expected_content_headers() {
		let file = BUNDLED_TEST_FILES.get_file("index.html").unwrap();
		let etag = ETag::from(&file);

		let response = ok(file, etag);
		let headers = response.headers();

		assert_eq!("text/html", headers[CONTENT_TYPE]);
		assert_eq!(file.contents().len().to_string(), headers[CONTENT_LENGTH])
	}

	#[test]
	fn ok_responses_should_have_an_etag_header() {
		let file = BUNDLED_TEST_FILES.get_file("index.html").unwrap();
		let etag = ETag::from(&file);

		let response = ok(file, etag);
		let headers = response.headers();

		assert_eq!(etag.to_string().as_bytes(), headers[ETAG].as_bytes())
	}

	#[tokio::test]
	async fn ok_response_should_contain_the_content() {
		let file = BUNDLED_TEST_FILES.get_file("index.html").unwrap();
		let etag = ETag::from(&file);

		let response = ok(file, etag);

		assert_eq!(file.contents(), content(response).await);
	}

	#[test]
	fn ok_response_should_have_the_correct_status_code() {
		let file = BUNDLED_TEST_FILES.get_file("index.html").unwrap();
		let etag = ETag::from(&file);
		let response = ok(file, etag);

		let status_code = response.status();

		assert_eq!(StatusCode::OK, status_code)
	}

	#[test]
	fn not_modified_response_should_have_the_correct_status_code() {
		let response = not_modified();

		let status_code = response.status();

		assert_eq!(StatusCode::NOT_MODIFIED, status_code);
	}

	#[tokio::test]
	async fn not_modified_response_should_have_explanatory_content() {
		let response = not_modified();

		assert_eq!("Not Modified", content(response).await);
	}

	#[test]
	fn not_found_response_should_have_the_correct_status_code() {
		let response = not_found();

		let status_code = response.status();

		assert_eq!(StatusCode::NOT_FOUND, status_code);
	}

	#[test]
	fn not_found_response_should_not_be_cached() {
		let response = not_found();

		let headers = response.headers();

		assert_eq!("no-cache", headers[CACHE_CONTROL]);
	}

	#[tokio::test]
	async fn not_found_response_should_have_explanatory_content() {
		let response = not_found();

		assert_eq!("Not Found", content(response).await);
	}

	async fn content(response: Response<Body>) -> Bytes {
		rweb::hyper::body::to_bytes(response.into_body()).await.unwrap()
	}
}
