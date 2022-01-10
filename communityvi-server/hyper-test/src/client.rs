use crate::request_builder::RequestBuilder;
use crate::{connection, serve, Connector, Host};
use hyper::body::HttpBody;
use hyper::service::Service;
use hyper::{http, Body, Method, Request, Response};
use std::error::Error;
use std::future::Future;
use std::sync::Arc;

mod aborting_join_handle;
use aborting_join_handle::AbortingJoinHandle;

#[derive(Clone)]
pub struct Client {
	pub(crate) client: hyper::Client<Connector, Body>,
	bind_host: Option<Host>,
	_server_handle: Arc<AbortingJoinHandle<hyper::Result<()>>>,
}

impl Client {
	/// Create a new [`Client`] from a service factory. This is the type you would pass to [`hyper::server::Builder::serve`].
	pub fn new<MakeService, MakeError, MakeFuture, HttpService, ResponseBody>(
		make_service: MakeService,
	) -> Result<Self, crate::Error>
	where
		MakeService: for<'a> Service<&'a connection::Connection, Response = HttpService, Error = MakeError, Future = MakeFuture>
			+ Send
			+ 'static,
		MakeError: Error + Send + Sync + 'static,
		MakeFuture: Future<Output = Result<HttpService, MakeError>> + Send + 'static,
		HttpService: Service<Request<Body>, Response = Response<ResponseBody>> + Send + 'static,
		HttpService::Error: Into<Box<dyn Error + Send + Sync>>,
		HttpService::Future: Send,
		ResponseBody: HttpBody + Send + 'static,
		ResponseBody::Data: Send,
		ResponseBody::Error: Error + Send + Sync,
	{
		let (client, server) = serve(make_service, None);
		let server_handle = tokio::spawn(server);

		Ok(Self {
			client,
			bind_host: None,
			_server_handle: Arc::new(server_handle.into()),
		})
	}

	/// Create a new [`Client`] from a service factory. This is the type you would pass to [`hyper::server::Builder::serve`].
	/// With bind host, the server binds to that URI and requests with relative URI are supported when not using
	/// the underlying [`hyper::Client`] directly.
	pub fn new_with_host<MakeService, MakeError, MakeFuture, HttpService, ResponseBody, BindHost>(
		make_service: MakeService,
		bind_host: BindHost,
	) -> Result<Self, crate::Error>
	where
		MakeService: for<'a> Service<&'a connection::Connection, Response = HttpService, Error = MakeError, Future = MakeFuture>
			+ Send
			+ 'static,
		MakeError: Error + Send + Sync + 'static,
		MakeFuture: Future<Output = Result<HttpService, MakeError>> + Send + 'static,
		HttpService: Service<Request<Body>, Response = Response<ResponseBody>> + Send + 'static,
		HttpService::Error: Into<Box<dyn Error + Send + Sync>>,
		HttpService::Future: Send,
		ResponseBody: HttpBody + Send + 'static,
		ResponseBody::Data: Send,
		ResponseBody::Error: Error + Send + Sync,
		Host: TryFrom<BindHost>,
		<Host as TryFrom<BindHost>>::Error: Into<crate::Error>,
	{
		let bind_host = Some(Host::try_from(bind_host).map_err(Into::into)?);

		let (client, server) = serve(make_service, bind_host.clone());
		let server_handle = tokio::spawn(server);

		Ok(Self {
			client,
			bind_host,
			_server_handle: Arc::new(server_handle.into()),
		})
	}

	/// Returns a copy of the underlying [`hyper::Client`].
	/// NOTE: This client will only work as long as an instance of this [`Client`] is still alive
	/// because otherwise the server will be dropped and the [`hyper::Client`] isn't connected
	/// to anything anymore.
	pub fn hyper_client(&self) -> hyper::Client<Connector, Body> {
		self.client.clone()
	}

	pub fn get<Uri>(&self, uri: Uri) -> RequestBuilder
	where
		http::Uri: TryFrom<Uri>,
		<http::Uri as TryFrom<Uri>>::Error: Into<http::Error>,
	{
		self.method(Method::GET, uri)
	}

	pub fn post<Uri>(&self, uri: Uri) -> RequestBuilder
	where
		http::Uri: TryFrom<Uri>,
		<http::Uri as TryFrom<Uri>>::Error: Into<http::Error>,
	{
		self.method(Method::POST, uri)
	}

	pub fn put<Uri>(&self, uri: Uri) -> RequestBuilder
	where
		http::Uri: TryFrom<Uri>,
		<http::Uri as TryFrom<Uri>>::Error: Into<http::Error>,
	{
		self.method(Method::PUT, uri)
	}

	pub fn delete<Uri>(&self, uri: Uri) -> RequestBuilder
	where
		http::Uri: TryFrom<Uri>,
		<http::Uri as TryFrom<Uri>>::Error: Into<http::Error>,
	{
		self.method(Method::DELETE, uri)
	}

	pub fn head<Uri>(&self, uri: Uri) -> RequestBuilder
	where
		http::Uri: TryFrom<Uri>,
		<http::Uri as TryFrom<Uri>>::Error: Into<http::Error>,
	{
		self.method(Method::HEAD, uri)
	}

	pub fn method<Uri>(&self, method: Method, uri: Uri) -> RequestBuilder
	where
		http::Uri: TryFrom<Uri>,
		<http::Uri as TryFrom<Uri>>::Error: Into<http::Error>,
	{
		let builder = RequestBuilder::new(self.clone()).method(method).uri(uri);
		match self.bind_host.clone() {
			Some(host) => builder.fallback_host(host),
			None => builder,
		}
	}

	pub fn build_request(&self) -> RequestBuilder {
		RequestBuilder::new(self.clone())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use futures_util::future;
	use serde::{Deserialize, Serialize};
	use std::collections::HashMap;
	use std::convert::Infallible;
	use std::pin::Pin;
	use std::task::{Context, Poll};

	#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
	struct Echo {
		uri: String,
		headers: HashMap<String, String>,
		method: String,
		content: Vec<u8>,
	}

	struct EchoService;

	impl Service<Request<Body>> for EchoService {
		type Response = Response<Body>;
		type Error = Infallible;
		type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

		fn poll_ready(&mut self, _context: &mut Context) -> Poll<Result<(), Self::Error>> {
			Poll::Ready(Ok(()))
		}

		fn call(&mut self, request: Request<Body>) -> Self::Future {
			Box::pin(async move {
				let uri = request.uri().to_string();
				let headers = request
					.headers()
					.iter()
					.map(|(name, value)| {
						(
							name.to_string(),
							value
								.to_str()
								.expect("Header can't be converted to string.")
								.to_string(),
						)
					})
					.collect();
				let method = request.method().to_string();

				let content = hyper::body::to_bytes(request.into_body())
					.await
					.expect("Failed to read request body.")
					.as_ref()
					.to_owned();
				let echo = Echo {
					uri,
					headers,
					method,
					content,
				};

				let json = serde_json::to_string(&echo).expect("Failed to create JSON.");

				Ok(Response::builder()
					.body(Body::from(json))
					.expect("Failed to build echo response."))
			})
		}
	}

	struct EchoMakeService;

	impl<T> Service<T> for EchoMakeService {
		type Response = EchoService;
		type Error = Infallible;
		type Future = future::Ready<Result<Self::Response, Self::Error>>;

		fn poll_ready(&mut self, _context: &mut Context) -> Poll<Result<(), Self::Error>> {
			Poll::Ready(Ok(()))
		}

		fn call(&mut self, _request: T) -> Self::Future {
			future::ready(Ok(EchoService))
		}
	}

	fn echo_client_without_bind_host() -> Client {
		Client::new(EchoMakeService).expect("Failed to create echo client.")
	}

	fn echo_client() -> Client {
		Client::new_with_host(EchoMakeService, "example.com").expect("Failed to create echo client with bind host.")
	}

	#[tokio::test]
	async fn client_without_bind_host_performs_requests() {
		let client = echo_client_without_bind_host();

		let mut response = client
			.get("https://example.com/some/test?query=1&other_query=2")
			.send()
			.await
			.expect("Failed to perform get request");
		let echo = response
			.deserialize_json::<Echo>()
			.await
			.expect("Failed to deserialize echo");

		let expected_echo = Echo {
			uri: "/some/test?query=1&other_query=2".to_string(),
			headers: headers([("host", "example.com")]),
			method: "GET".to_string(),
			content: Vec::new(),
		};
		assert_eq!(expected_echo, echo, "The server didn't echo back the request we sent.");
	}

	#[tokio::test]
	async fn client_performs_requests() {
		let client = echo_client();

		let mut response = client
			.get("/some/test?query=value")
			.send()
			.await
			.expect("Failed to perform get request");
		let echo = response
			.deserialize_json::<Echo>()
			.await
			.expect("Failed to deserialize echo");

		let expected_echo = Echo {
			uri: "/some/test?query=value".to_string(),
			headers: headers([("host", "example.com")]),
			method: "GET".to_string(),
			content: Vec::new(),
		};
		assert_eq!(expected_echo, echo, "The server didn't echo back the request we sent.")
	}

	fn headers(headers: impl IntoIterator<Item = (&'static str, &'static str)>) -> HashMap<String, String> {
		headers
			.into_iter()
			.map(|(name, value)| (name.to_string(), value.to_string()))
			.collect()
	}
}
