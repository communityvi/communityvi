use crate::connection::Connection;
use crate::error::InternalError;
use crate::Host;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use hyper::service::Service;
use hyper::Uri;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Connector {
	sender: mpsc::Sender<Connection>,
	bind_host: Option<Host>,
}

impl Connector {
	pub(crate) fn new(sender: mpsc::Sender<Connection>) -> Self {
		Self {
			sender,
			bind_host: None,
		}
	}

	pub(crate) fn bind_to_host(self, host: Host) -> Self {
		Self {
			sender: self.sender,
			bind_host: Some(host),
		}
	}
}

impl Service<Uri> for Connector {
	type Response = Connection;
	type Error = crate::Error;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _context: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, request: Uri) -> Self::Future {
		let sender = self.sender.clone();

		let host_result: Result<(), crate::Error> = if let Some(host) = &self.bind_host {
			request
				.host()
				.ok_or_else(|| {
					InternalError::Connection(format!("Request is missing a host. Server was bound to '{host}'")).into()
				})
				.and_then(|request_host| {
					if request_host == host.as_ref() {
						Ok(())
					} else {
						Err(InternalError::Connection(format!(
							"No server available for the host '{request_host}', server was bound to '{host}'"
						))
						.into())
					}
				})
		} else {
			Ok(())
		};

		async move {
			// defer the error reporting to inside of the future
			host_result?;

			let (client_connection, server_connection) = Connection::new();
			match sender.send(server_connection).await {
				Ok(()) => Ok(client_connection),
				Err(_) => Err(InternalError::Connection("Failed to establish connection.".into()).into()),
			}
		}
		.boxed()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use futures_util::future::poll_fn;

	use crate::connection::test::ensure_connection_works;
	use hyper::client::connect::Connect;
	use static_assertions::assert_impl_all;

	assert_impl_all!(Connector: Connect, Clone);

	#[tokio::test]
	async fn should_connect_with_unbound_host() {
		let (sender, receiver) = mpsc::channel(1);
		let connector = Connector::new(sender);

		test_connector(connector, receiver, "*").await;
	}

	#[tokio::test]
	async fn should_connect_when_requesting_correct_bound_host() {
		let (sender, receiver) = mpsc::channel(1);
		let connector = Connector::new(sender).bind_to_host(Host::try_from("localhost").unwrap());

		test_connector(connector, receiver, "https://localhost/test").await;
	}

	#[tokio::test]
	async fn should_fail_to_connect_when_requesting_other_host_than_bound() {
		let (sender, _receiver) = mpsc::channel(1);
		let mut connector = Connector::new(sender).bind_to_host(Host::try_from("localhost").unwrap());

		ensure_ready(&mut connector).await;
		let _ = connector
			.call(Uri::from_static("https://example.com/test"))
			.await
			.map(|_| panic!("Didn't fail to connect when it should have."));
	}

	#[tokio::test]
	async fn should_fail_to_connect_when_requesting_no_host_when_one_is_bound() {
		let (sender, _receiver) = mpsc::channel(1);
		let mut connector = Connector::new(sender).bind_to_host(Host::try_from("localhost").unwrap());

		ensure_ready(&mut connector).await;
		let _ = connector
			.call(Uri::from_static("*"))
			.await
			.map(|_| panic!("Didn't fail to connect when it should have."));
	}

	#[tokio::test]
	async fn should_fail_to_connect_when_receiver_is_dropped() {
		let (sender, receiver) = mpsc::channel(1);
		drop(receiver);

		let mut connector = Connector::new(sender);

		ensure_ready(&mut connector).await;
		let _ = connector
			.call(Uri::from_static("*"))
			.await
			.map(|_| panic!("Didn't fail to connect when it should have."));
	}

	async fn test_connector(mut connector: Connector, mut receiver: mpsc::Receiver<Connection>, uri: &'static str) {
		ensure_ready(&mut connector).await;

		let client = connector.call(Uri::from_static(uri)).await.expect("Failed to connect");
		let server = receiver.recv().await.expect("Failed to receive server connection.");

		ensure_connection_works(client, server).await;
	}

	async fn ensure_ready(connector: &mut Connector) {
		poll_fn(|context| connector.poll_ready(context))
			.await
			.expect("Checking service readiness failed.");
	}
}
