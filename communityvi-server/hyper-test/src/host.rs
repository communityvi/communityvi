use crate::error::InternalError;
use hyper::http::uri;
use hyper::http::uri::Authority;
use hyper::Uri;
use std::fmt::{Display, Formatter};
use std::str::{from_utf8, FromStr};

/// Validated URI host.
#[derive(Clone, Debug)]
pub struct Host(String);

impl Host {
	fn from_authority_with_only_host(authority: &Authority) -> Result<Self, crate::Error> {
		if authority.host() != authority.as_ref() {
			return Err(InternalError::InvalidHost(format!("{authority} is more than just a host.")).into());
		}

		Ok(Self(authority.host().to_string()))
	}
}

impl Display for Host {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		self.0.fmt(formatter)
	}
}

impl AsRef<str> for Host {
	fn as_ref(&self) -> &str {
		&self.0
	}
}

impl From<Host> for String {
	fn from(host: Host) -> Self {
		host.0
	}
}

impl From<&uri::Authority> for Host {
	fn from(authority: &Authority) -> Self {
		Self(authority.host().to_string())
	}
}

impl TryFrom<Uri> for Host {
	type Error = crate::Error;

	fn try_from(uri: Uri) -> Result<Self, Self::Error> {
		let parts = uri.clone().into_parts();
		let authority = parts
			.authority
			.ok_or_else(|| InternalError::InvalidHost(format!("URI {uri} doesn't have a host")))?;
		Host::from_authority_with_only_host(&authority)
	}
}

impl TryFrom<&str> for Host {
	type Error = crate::Error;

	fn try_from(host: &str) -> Result<Self, Self::Error> {
		let authority = Authority::from_str(host).map_err(|error| InternalError::InvalidHost(error.to_string()))?;
		Host::from_authority_with_only_host(&authority)
	}
}

impl TryFrom<String> for Host {
	type Error = crate::Error;

	fn try_from(host: String) -> Result<Self, Self::Error> {
		Self::try_from(host.as_str())
	}
}

impl TryFrom<&[u8]> for Host {
	type Error = crate::Error;

	fn try_from(host: &[u8]) -> Result<Self, Self::Error> {
		let text = from_utf8(host).map_err(|error| InternalError::InvalidHost(error.to_string()))?;
		Self::try_from(text)
	}
}
