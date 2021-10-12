use mime_guess::MimeGuess;
use rust_embed::{EmbeddedFile, RustEmbed};
use rweb::http::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use rweb::http::{HeaderMap, Response, StatusCode};
use rweb::hyper::Body;

#[allow(unused)]
#[derive(Clone)]
pub struct BundledFileHandler {
	file_getter: fn(path: &str) -> Option<EmbeddedFile>,
}

#[allow(unused)]
impl BundledFileHandler {
	/// Creates a new [`BundledFileHandler`] from a [`RustEmbed`] asset type, erasing the type in the process.
	pub fn new<Bundle: RustEmbed>() -> Self {
		Self {
			file_getter: Bundle::get,
		}
	}

	pub fn request(&self, path: &str, request_headers: &HeaderMap) -> Response<Body> {
		let file = match self.look_up_file_falling_back_to_index_html(path) {
			Some(file) => file,
			None => return not_found(),
		};

		if file.is_cached(request_headers) {
			return not_modified();
		}

		file.to_response()
	}

	fn look_up_file_falling_back_to_index_html(&self, path: &str) -> Option<BundledFile> {
		match self.look_up_file(path) {
			Some(file) => Some(file),
			None => self.look_up_file(&format!("{}/index.html", path)),
		}
	}

	fn look_up_file(&self, path: &str) -> Option<BundledFile> {
		let path = normalize_path(path);
		(self.file_getter)(path).map(|file| BundledFile {
			file,
			path: path.to_string(),
		})
	}
}

pub struct BundledFile {
	file: EmbeddedFile,
	path: String,
}

impl BundledFile {
	fn etag(&self) -> String {
		hex::encode(&self.file.metadata.sha256_hash())
	}

	fn is_cached(&self, request_headers: &HeaderMap) -> bool {
		match request_headers.get(IF_NONE_MATCH) {
			Some(if_none_match) => &self.etag() == if_none_match,
			None => false,
		}
	}

	fn to_response(&self) -> Response<Body> {
		let mime = MimeGuess::from_path(&self.path).first_or_octet_stream();
		Response::builder()
			.status(StatusCode::OK)
			.header(CONTENT_TYPE, mime.as_ref())
			.header(CONTENT_LENGTH, self.file.data.len())
			// Tell browsers to always make the request with If-None-Match instead
			// of relying on a maximum age.
			.header(CACHE_CONTROL, "must-revalidate")
			.header(ETAG, self.etag())
			.body(Body::from(self.file.data.clone()))
			.unwrap()
	}

	#[cfg(test)]
	fn content(&self) -> &[u8] {
		self.file.data.as_ref()
	}
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
	use super::*;
	use rweb::http::HeaderValue;
	use rweb::hyper::body::Bytes;

	#[derive(RustEmbed)]
	#[folder = "$CARGO_MANIFEST_DIR/test/bundled_files"]
	struct TestBundle;

	#[tokio::test]
	async fn request_handler_should_return_files() {
		let index = bundled_file("index.html");

		let response = test_handler().request("index.html", &HeaderMap::default());
		let status_code = response.status();
		let content = content(response).await;

		assert_eq!(index.content(), content);
		assert_eq!(StatusCode::OK, status_code);
	}

	#[test]
	fn request_handler_should_reply_with_not_found_if_file_is_not_found() {
		let response = test_handler().request("nonexistent", &HeaderMap::default());

		assert_eq!(StatusCode::NOT_FOUND, response.status());
	}

	#[test]
	fn request_handler_should_reply_with_not_modified_if_etag_matches() {
		const PATH: &str = "about/index.html";
		let uncached_response = test_handler().request(PATH, &HeaderMap::default());
		let etag = uncached_response.headers()[ETAG].as_bytes();

		let mut request_headers = HeaderMap::new();
		request_headers.insert(IF_NONE_MATCH, HeaderValue::from_bytes(etag).unwrap());

		let cached_response = test_handler().request(PATH, &request_headers);

		assert_eq!(StatusCode::NOT_MODIFIED, cached_response.status());
	}

	#[test]
	fn request_handler_should_return_file_with_not_modified_if_etag_does_not_match() {
		const PATH: &str = "about/index.html";

		let mut request_headers = HeaderMap::new();
		request_headers.insert(IF_NONE_MATCH, "wrong_etag".parse().unwrap());

		let response = test_handler().request(PATH, &request_headers);

		assert_eq!(StatusCode::OK, response.status());
	}

	#[test]
	fn normalize_path_should_strip_slashes() {
		assert_eq!("", normalize_path("/"));
		assert_eq!("index.html", normalize_path("index.html/"));
		assert_eq!("index.html", normalize_path("/index.html"));
		assert_eq!("index.html", normalize_path("/index.html/"));
	}

	#[tokio::test]
	async fn request_handler_should_normalize_path() {
		let index = bundled_file("index.html");

		let response = test_handler().request("/index.html/", &HeaderMap::default());

		assert_eq!(index.content(), content(response).await);
	}

	#[test]
	fn ok_responses_should_contain_the_expected_cache_control_header() {
		let file = bundled_file("index.html");

		let response = file.to_response();
		let headers = response.headers();

		assert_eq!("must-revalidate", headers[CACHE_CONTROL])
	}

	#[test]
	fn ok_responses_should_contain_the_expected_content_headers() {
		let file = bundled_file("index.html");

		let response = file.to_response();
		let headers = response.headers();

		assert_eq!("text/html", headers[CONTENT_TYPE]);
		assert_eq!(file.content().len().to_string(), headers[CONTENT_LENGTH])
	}

	#[test]
	fn ok_responses_should_have_an_etag_header() {
		let file = bundled_file("index.html");

		let response = file.to_response();
		let headers = response.headers();

		assert_eq!(file.etag().as_bytes(), headers[ETAG].as_bytes())
	}

	#[tokio::test]
	async fn ok_response_should_contain_the_content() {
		let file = bundled_file("index.html");

		let response = file.to_response();

		assert_eq!(file.content(), content(response).await);
	}

	#[test]
	fn ok_response_should_have_the_correct_status_code() {
		let file = bundled_file("index.html");

		let response = file.to_response();
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

	fn test_handler() -> BundledFileHandler {
		BundledFileHandler::new::<TestBundle>()
	}

	fn bundled_file(path: &str) -> BundledFile {
		let file = TestBundle::get(path).unwrap();
		BundledFile {
			file,
			path: path.to_string(),
		}
	}

	async fn content(response: Response<Body>) -> Bytes {
		rweb::hyper::body::to_bytes(response.into_body()).await.unwrap()
	}
}
