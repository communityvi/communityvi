use axum::body::{Body, HttpBody};
use axum::http::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LAST_MODIFIED};
use axum::http::{HeaderMap, Request, Response, StatusCode};
use bytes::Bytes;
use chrono::{DateTime, TimeZone, Utc};
use mime::Mime;
use mime_guess::MimeGuess;
use rust_embed::{EmbeddedFile, RustEmbed};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower_service::Service;

#[allow(unused)]
#[derive(Clone)]
pub struct BundledFileHandler {
	files_by_path: Arc<HashMap<Cow<'static, str>, BundledFile>>,
}

#[allow(unused)]
impl BundledFileHandler {
	pub fn builder() -> BundledFileHandlerBuilder {
		BundledFileHandlerBuilder::default()
	}

	pub fn request(&self, path: &str, request_headers: &HeaderMap) -> Response<Body> {
		let file = match self.look_up_file_falling_back_to_index_html(path) {
			Some(file) => file,
			None => return not_found(),
		};

		if file.is_cached(request_headers) {
			return not_modified();
		}

		file.as_response()
	}

	fn look_up_file_falling_back_to_index_html(&self, path: &str) -> Option<&BundledFile> {
		match self.look_up_file(path) {
			Some(file) => Some(file),
			None => self.look_up_file(&format!("{path}/index.html")),
		}
	}

	fn look_up_file(&self, path: &str) -> Option<&BundledFile> {
		let path = normalize_path(path);
		self.files_by_path.get(path)
	}
}

#[derive(Default)]
pub struct BundledFileHandlerBuilder {
	files_by_path: HashMap<Cow<'static, str>, BundledFile>,
}

impl BundledFileHandlerBuilder {
	/// Add files from a [`RustEmbed`] asset type, erasing the type in the process.
	pub fn with_rust_embed<Bundle: RustEmbed>(mut self) -> Self {
		self.files_by_path.extend(
			Bundle::iter()
				.filter_map(|path| Bundle::get(&path).map(|file| (path, file)))
				.map(|(path, file)| {
					let file = BundledFile::from_embedded_file(&path, file);
					(path, file)
				}),
		);
		self
	}

	pub fn with_file(mut self, path: Cow<'static, str>, content: impl Into<Bytes>) -> Self {
		let file = BundledFile::new(&path, content);
		self.files_by_path.insert(path, file);
		self
	}

	pub fn build(self) -> BundledFileHandler {
		BundledFileHandler {
			files_by_path: Arc::new(self.files_by_path),
		}
	}
}

impl<B> Service<Request<B>> for BundledFileHandler
where
	B: HttpBody + Send + 'static,
{
	type Response = Response<Body>;
	type Error = Infallible;
	type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, request: Request<B>) -> Self::Future {
		let path = request.uri().path();
		let headers = request.headers();
		std::future::ready(Ok(self.request(path, headers)))
	}
}

#[derive(Clone)]
struct BundledFile {
	content: Bytes,
	hash: [u8; 32],
	mime_type: Mime,
	last_modified: Option<DateTime<Utc>>,
}

impl BundledFile {
	fn new(path: &str, content: impl Into<Bytes>) -> Self {
		let content = content.into();
		let hash = hash_sha256(&content);
		let mime_type = MimeGuess::from_path(path).first_or_octet_stream();
		Self {
			content,
			hash,
			mime_type,
			last_modified: None,
		}
	}

	fn from_embedded_file(path: &str, file: EmbeddedFile) -> Self {
		let content = match file.data {
			Cow::Borrowed(slice) => slice.into(),
			Cow::Owned(owned) => owned.into(),
		};
		let mime_type = MimeGuess::from_path(path).first_or_octet_stream();
		Self {
			content,
			hash: file.metadata.sha256_hash(),
			mime_type,
			last_modified: file
				.metadata
				.last_modified()
				.and_then(|timestamp| i64::try_from(timestamp).ok())
				.and_then(|timestamp| Utc.timestamp_opt(timestamp, 0).single()),
		}
	}

	fn as_response(&self) -> Response<Body> {
		let builder = Response::builder()
			.status(StatusCode::OK)
			.header(CONTENT_TYPE, self.mime_type.as_ref())
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
		format!(r#""{}""#, hex::encode(self.hash))
	}

	fn is_cached(&self, request_headers: &HeaderMap) -> bool {
		match request_headers.get(IF_NONE_MATCH) {
			Some(if_none_match) => &self.etag() == if_none_match,
			None => false,
		}
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

fn hash_sha256(bytes: &[u8]) -> [u8; 32] {
	let mut hasher = Sha256::default();
	hasher.update(bytes);
	hasher.finalize().into()
}

#[cfg(test)]
mod test {
	use super::*;
	use axum::body::Bytes;
	use axum::http::HeaderValue;
	use hyper_test::hyper;

	#[derive(RustEmbed)]
	#[folder = "$CARGO_MANIFEST_DIR/test/bundled_files"]
	struct TestBundle;

	#[tokio::test]
	async fn request_handler_should_return_files() {
		let index = bundled_file("index.html");
		let expected_content = index.content;

		let response = test_handler().request("index.html", &HeaderMap::default());
		let status_code = response.status();
		let content = response_content(response).await;

		assert_eq!(expected_content.as_ref(), content);
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
		let content = index.content;

		let response = test_handler().request("/index.html/", &HeaderMap::default());

		assert_eq!(content.as_ref(), response_content(response).await);
	}

	#[ignore = "../ in paths is currently unsupported"]
	#[tokio::test]
	async fn request_handler_should_work_with_double_dot() {
		let index = bundled_file("index.html");
		let content = index.content;

		let response = test_handler().request("foo/../index.html", &HeaderMap::default());

		assert_eq!(content.as_ref(), response_content(response).await);
	}

	#[test]
	fn ok_responses_should_contain_the_expected_cache_control_header() {
		let file = bundled_file("index.html");

		let response = file.as_response();
		let headers = response.headers();

		assert_eq!("must-revalidate", headers[CACHE_CONTROL]);
	}

	#[test]
	fn ok_responses_should_contain_the_expected_content_headers() {
		let file = bundled_file("index.html");
		let content = file.content.clone();

		let response = file.as_response();
		let headers = response.headers();

		assert_eq!("text/html", headers[CONTENT_TYPE]);
		assert_eq!(content.len().to_string(), headers[CONTENT_LENGTH]);
	}

	#[test]
	fn ok_responses_should_have_an_etag_header() {
		let file = bundled_file("index.html");
		let etag = file.etag();

		let response = file.as_response();
		let headers = response.headers();

		assert_eq!(etag.as_bytes(), headers[ETAG].as_bytes());
	}

	#[test]
	fn ok_responses_should_have_a_last_modified_header() {
		let file = bundled_file("index.html");

		let response = file.as_response();
		let headers = response.headers();

		assert!(headers.contains_key(LAST_MODIFIED));
	}

	#[test]
	fn last_modified_should_have_the_expected_format() {
		let date_time = Utc
			.with_ymd_and_hms(2021, 10, 12, 13, 37, 42)
			.single()
			.unwrap_or_else(|| unreachable!("Hardcoded date was not valid"));

		let last_modified = last_modified_header_value(date_time);

		assert_eq!("Tue, 12 Oct 2021 13:37:42 GMT", last_modified);
	}

	#[tokio::test]
	async fn ok_response_should_contain_the_content() {
		let file = bundled_file("index.html");
		let content = file.content.clone();

		let response = file.as_response();

		assert_eq!(content.as_ref(), response_content(response).await);
	}

	#[test]
	fn ok_response_should_have_the_correct_status_code() {
		let file = bundled_file("index.html");

		let response = file.as_response();
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

		assert_eq!("Not Modified", response_content(response).await);
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

		assert_eq!("Not Found", response_content(response).await);
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
		BundledFileHandler::builder().with_rust_embed::<TestBundle>().build()
	}

	fn bundled_file(path: &'static str) -> BundledFile {
		let file = TestBundle::get(path).unwrap();
		BundledFile::from_embedded_file(path, file)
	}

	async fn response_content(response: Response<Body>) -> Bytes {
		hyper::body::to_bytes(response.into_body()).await.unwrap()
	}
}
