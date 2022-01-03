use chrono::{DateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use mime_guess::MimeGuess;
use parking_lot::RwLock;
use rust_embed::{EmbeddedFile, RustEmbed};
use rweb::http::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LAST_MODIFIED};
use rweb::http::{HeaderMap, Response, StatusCode};
use rweb::hyper::Body;
use rweb::path::Tail;
use rweb::{filters, Filter};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};

#[allow(unused)]
#[derive(Clone)]
pub struct BundledFileHandler {
	file_getter: fn(path: &str) -> Option<BundledFile>,
}

#[allow(unused)]
impl BundledFileHandler {
	/// Creates a new [`BundledFileHandler`] from a [`RustEmbed`] asset type, erasing the type in the process.
	pub fn new_with_rust_embed<Bundle: RustEmbed>() -> Self {
		Self {
			file_getter: |path| Bundle::get(path).map(|file| BundledFile::from_embedded_file(path, file)),
		}
	}

	#[cfg(feature = "api-docs")]
	/// Creates a new [`BundledFileHandler`] from a [`rust_embed5::RustEmbed`] asset type, erasing the type in the process.
	pub fn new_with_rust_embed5<Bundle: rust_embed5::RustEmbed>() -> Self {
		Self {
			file_getter: |path| Bundle::get(path).map(|content| BundledFile::new(path, content)),
		}
	}

	pub fn into_rweb_filter(self) -> impl Filter<Extract = (Response<Body>,), Error = Infallible> {
		filters::path::tail()
			.and(filters::header::headers_cloned())
			.map(move |path: Tail, headers: HeaderMap| self.request(path.as_str(), &headers))
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
		(self.file_getter)(path)
	}
}

pub struct BundledFile {
	path: String,
	content: Cow<'static, [u8]>,
	hash: [u8; 32],
	last_modified: Option<DateTime<Utc>>,
}

impl BundledFile {
	fn new(path: &str, content: Cow<'static, [u8]>) -> Self {
		let hash = cached_sha256(path.as_ref(), &content);
		Self {
			path: path.into(),
			content,
			hash,
			last_modified: None,
		}
	}

	fn from_embedded_file(path: &str, file: EmbeddedFile) -> Self {
		Self {
			path: path.to_string(),
			content: file.data,
			hash: file.metadata.sha256_hash(),
			last_modified: file
				.metadata
				.last_modified()
				.and_then(|timestamp| i64::try_from(timestamp).ok())
				.map(|timestamp| Utc.timestamp(timestamp, 0)),
		}
	}

	fn to_response(&self) -> Response<Body> {
		let mime = MimeGuess::from_path(&self.path).first_or_octet_stream();
		let builder = Response::builder()
			.status(StatusCode::OK)
			.header(CONTENT_TYPE, mime.as_ref())
			.header(CONTENT_LENGTH, self.content.len())
			// Tell browsers to always make the request with If-None-Match instead
			// of relying on a maximum age.
			.header(CACHE_CONTROL, "must-revalidate")
			.header(ETAG, self.etag());

		let builder = if let Some(last_modified) = self.last_modified.map(last_modified_header_value) {
			builder.header(LAST_MODIFIED, last_modified)
		} else {
			builder
		};

		builder.body(Body::from(self.content.clone())).unwrap()
	}

	fn etag(&self) -> String {
		format!(r#""{}""#, hex::encode(&self.hash))
	}

	fn is_cached(&self, request_headers: &HeaderMap) -> bool {
		match request_headers.get(IF_NONE_MATCH) {
			Some(if_none_match) => &self.etag() == if_none_match,
			None => false,
		}
	}

	#[cfg(test)]
	fn content(&self) -> &[u8] {
		self.content.as_ref()
	}
}

fn last_modified_header_value(date_time: DateTime<Utc>) -> String {
	// https://httpwg.org/specs/rfc7231.html#http.date
	date_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
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

fn cached_sha256(path: &Path, bytes: &[u8]) -> [u8; 32] {
	lazy_static! {
		static ref CACHE: RwLock<HashMap<PathBuf, [u8; 32]>> = RwLock::default();
	};

	{
		let cache = CACHE.read();
		if let Some(&hash) = cache.get(path) {
			return hash;
		}
	}

	*CACHE.write().entry(path.into()).or_insert_with(|| {
		let mut hasher = Sha256::default();
		hasher.update(bytes);
		hasher.finalize().into()
	})
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

		assert_eq!("must-revalidate", headers[CACHE_CONTROL]);
	}

	#[test]
	fn ok_responses_should_contain_the_expected_content_headers() {
		let file = bundled_file("index.html");

		let response = file.to_response();
		let headers = response.headers();

		assert_eq!("text/html", headers[CONTENT_TYPE]);
		assert_eq!(file.content().len().to_string(), headers[CONTENT_LENGTH]);
	}

	#[test]
	fn ok_responses_should_have_an_etag_header() {
		let file = bundled_file("index.html");

		let response = file.to_response();
		let headers = response.headers();

		assert_eq!(file.etag().as_bytes(), headers[ETAG].as_bytes());
	}

	#[test]
	fn ok_responses_should_have_a_last_modified_header() {
		let file = bundled_file("index.html");

		let response = file.to_response();
		let headers = response.headers();

		assert!(headers.contains_key(LAST_MODIFIED));
	}

	#[test]
	fn last_modified_should_have_the_expected_format() {
		let date_time = Utc.ymd(2021, 10, 12).and_hms(13, 37, 42);

		let last_modified = last_modified_header_value(date_time);

		assert_eq!("Tue, 12 Oct 2021 13:37:42 GMT", last_modified);
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

		assert_eq!(StatusCode::OK, status_code);
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

	#[test]
	fn bundled_file_should_format_etag_properly() {
		let file = bundled_file("index.html");
		let etag = file.etag();

		assert!(etag.starts_with('"'));
		assert!(etag.ends_with('"'));
		assert!(!etag.trim_matches('"').contains('"'));
	}

	fn test_handler() -> BundledFileHandler {
		BundledFileHandler::new_with_rust_embed::<TestBundle>()
	}

	fn bundled_file(path: &str) -> BundledFile {
		let file = TestBundle::get(path).unwrap();
		BundledFile::from_embedded_file(path, file)
	}

	async fn content(response: Response<Body>) -> Bytes {
		rweb::hyper::body::to_bytes(response.into_body()).await.unwrap()
	}
}
