use crate::connection::Connection;
use futures_util::ready;
use hyper::server::accept::Accept;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

pub struct ConnectionIncoming {
	receiver: mpsc::Receiver<Connection>,
}

impl ConnectionIncoming {
	pub(crate) fn new(receiver: mpsc::Receiver<Connection>) -> Self {
		Self { receiver }
	}
}

impl Accept for ConnectionIncoming {
	type Conn = Connection;
	type Error = Infallible;

	fn poll_accept(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
		let mut receiver = Pin::new(&mut self.receiver);
		Poll::Ready(ready!(receiver.poll_recv(context)).map(Ok))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::connection::test::ensure_connection_works;
	use futures_util::future::poll_fn;

	#[tokio::test]
	async fn should_accept_incoming_connections() {
		let (sender, receiver) = mpsc::channel(1);
		let (client, server) = Connection::new();
		sender
			.send(server)
			.await
			.unwrap_or_else(|_| panic!("Failed to send server connection"));

		let mut incoming = ConnectionIncoming::new(receiver);

		let server = poll_fn(|context| Pin::new(&mut incoming).poll_accept(context))
			.await
			.expect("Unexpectedly closed.")
			.expect("Didn't receive connection");

		ensure_connection_works(client, server).await;
	}

	#[tokio::test]
	async fn should_propagate_closed_channel() {
		let (sender, receiver) = mpsc::channel(1);
		drop(sender);

		let mut incoming = ConnectionIncoming::new(receiver);

		let connection = poll_fn(|context| Pin::new(&mut incoming).poll_accept(context)).await;
		assert!(
			connection.is_none(),
			"Incoming connection wasn't closed where it should have been."
		);
	}
}
