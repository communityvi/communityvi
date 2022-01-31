use crate::client::Client;
use crate::error::InternalError;
use crate::response::Response;
use crate::Host;
use base64::display::Base64Display;
use base64::{CharacterSet, Config};
use hyper::header::{HeaderName, HeaderValue};
use hyper::http::header::AUTHORIZATION;
use hyper::http::uri;
use hyper::{http, HeaderMap};
use hyper::{Body, Uri};
use std::fmt::Display;
use std::future::Future;
use std::mem::swap;

#[derive(Clone)]
pub struct RequestBuilder {
	client: Client,
	fallback_host: Option<Host>,
	components: Result<RequestComponents, crate::Error>,
}

#[derive(Clone, Default)]
struct RequestComponents {
	headers: HeaderMap,
	method: http::Method,
	uri: http::Uri,
}

impl RequestBuilder {
	pub(crate) fn new(client: Client) -> Self {
		Self {
			client,
			fallback_host: None,
			components: Ok(Default::default()),
		}
	}

	pub fn fallback_host(self, host: Host) -> Self {
		Self {
			client: self.client,
			fallback_host: Some(host),
			components: self.components,
		}
	}

	/// Add a header to this request.
	pub fn header<Name, Value>(mut self, name: Name, value: Value) -> Self
	where
		HeaderName: TryFrom<Name>,
		<HeaderName as TryFrom<Name>>::Error: Into<http::Error>,
		HeaderValue: TryFrom<Value>,
		<HeaderValue as TryFrom<Value>>::Error: Into<http::Error>,
	{
		self.components = self.components.and_then(move |mut components| {
			let name = HeaderName::try_from(name).map_err(Into::into)?;
			let value = HeaderValue::try_from(value).map_err(Into::into)?;
			components.headers.insert(name, value);
			Ok(components)
		});
		self
	}

	/// Add multiple headers to this request.
	pub fn headers(mut self, headers: HeaderMap) -> Self {
		if let Ok(components) = self.components.as_mut() {
			components.headers.extend(headers);
		}
		self
	}

	pub fn basic_auth<User, Password>(self, user: User, password: Password) -> Self
	where
		User: AsRef<[u8]>,
		Password: AsRef<[u8]>,
	{
		const SEPARATOR: u8 = b':';
		let mut user_pass = Vec::with_capacity(user.as_ref().len() + [SEPARATOR].len() + password.as_ref().len());
		user_pass.extend_from_slice(user.as_ref());
		user_pass.push(SEPARATOR);
		user_pass.extend_from_slice(password.as_ref());

		self.header(
			AUTHORIZATION,
			format!(
				"Basic {}",
				Base64Display::with_config(&user_pass, Config::new(CharacterSet::Standard, false))
			),
		)
	}

	pub fn bearer_auth<Token: Display>(self, token: Token) -> Self {
		self.header(AUTHORIZATION, format!("Bearer {token}"))
	}

	/// Set the request method for the request. Defaults to GET.
	pub fn method<Method>(mut self, method: Method) -> Self
	where
		http::Method: TryFrom<Method>,
		<http::Method as TryFrom<Method>>::Error: Into<http::Error>,
	{
		self.components = self.components.and_then(move |mut components| {
			let method = http::Method::try_from(method).map_err(Into::into)?;
			components.method = method;
			Ok(components)
		});
		self
	}

	/// Set the request URI. Defaults to `'/'`
	pub fn uri<Uri>(mut self, uri: Uri) -> Self
	where
		http::Uri: TryFrom<Uri>,
		<http::Uri as TryFrom<Uri>>::Error: Into<http::Error>,
	{
		self.components = self.components.and_then(move |mut components| {
			components.uri = TryFrom::try_from(uri).map_err(Into::into)?;
			Ok(components)
		});
		self
	}

	pub fn send(&self) -> impl Future<Output = Result<Response, crate::Error>> + Send + Sync + 'static {
		let Self {
			client,
			fallback_host,
			components,
		} = self.clone();
		let result = components.and_then(
			|RequestComponents {
			     mut headers,
			     method,
			     uri,
			 }| {
				let uri = if let Some(fallback) = fallback_host {
					uri_with_fallback(uri, fallback)?
				} else {
					uri
				};

				let mut request_builder = http::Request::builder().method(method).uri(uri);
				if let Some(request_headers) = request_builder.headers_mut() {
					swap(&mut headers, request_headers);
				}

				Ok(request_builder.body(Body::empty())?)
			},
		);

		async move {
			let request = result?;
			Ok(client
				.client
				.request(request)
				.await
				.map_err(|error| InternalError::Request(error.to_string()))?
				.into())
		}
	}
}

fn uri_with_fallback(uri: Uri, fallback_host: Host) -> http::Result<Uri> {
	match uri.clone().into_parts() {
		mut parts @ uri::Parts {
			scheme: None,
			authority: None,
			..
		} => {
			parts.scheme = Some(uri::Scheme::HTTPS);
			parts.authority = Some(uri::Authority::try_from(fallback_host.as_ref())?);
			Ok(Uri::from_parts(parts)?)
		}
		_ => Ok(uri),
	}
}
