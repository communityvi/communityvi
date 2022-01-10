use futures_util::ready;
use hyper::body::Bytes;
use hyper::client::connect::Connected;
use std::io;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tokio::sync::mpsc::unbounded_channel;

pub mod connection_incoming;
pub mod connector;

pub struct Connection {
	sender: mpsc::UnboundedSender<Bytes>,
	receiver: mpsc::UnboundedReceiver<Bytes>,
	receive_buffer: Bytes,
}

impl Connection {
	pub fn new() -> (Connection, Connection) {
		let (request_sender, request_receiver) = unbounded_channel();
		let (response_sender, response_receiver) = unbounded_channel();

		(
			Self {
				sender: request_sender,
				receiver: response_receiver,
				receive_buffer: Bytes::new(),
			},
			Self {
				sender: response_sender,
				receiver: request_receiver,
				receive_buffer: Bytes::new(),
			},
		)
	}
}

impl AsyncRead for Connection {
	fn poll_read(self: Pin<&mut Self>, context: &mut Context, read_buffer: &mut ReadBuf) -> Poll<std::io::Result<()>> {
		let this = self.get_mut();

		if this.receive_buffer.is_empty() {
			match ready!(this.receiver.poll_recv(context)) {
				Some(bytes) => this.receive_buffer = bytes,
				None => {
					return Poll::Ready(Err(io::Error::new(
						ErrorKind::BrokenPipe,
						"Channel closed.".to_string(),
					)))
				}
			}
		}

		let count = read_buffer.remaining().min(this.receive_buffer.len());
		let buffer_to_read = this.receive_buffer.split_to(count);
		read_buffer.put_slice(&buffer_to_read);

		Poll::Ready(Ok(()))
	}
}

impl AsyncWrite for Connection {
	fn poll_write(self: Pin<&mut Self>, _context: &mut Context, buffer: &[u8]) -> Poll<Result<usize, io::Error>> {
		if self.get_mut().sender.send(Bytes::copy_from_slice(buffer)).is_err() {
			return Poll::Ready(Err(io::Error::new(
				io::ErrorKind::BrokenPipe,
				"Channel closed.".to_string(),
			)));
		}

		Poll::Ready(Ok(buffer.len()))
	}

	fn poll_flush(self: Pin<&mut Self>, _context: &mut Context) -> Poll<Result<(), io::Error>> {
		Poll::Ready(Ok(()))
	}

	fn poll_shutdown(self: Pin<&mut Self>, _context: &mut Context) -> Poll<Result<(), io::Error>> {
		Poll::Ready(Ok(()))
	}
}

impl hyper::client::connect::Connection for Connection {
	fn connected(&self) -> Connected {
		Connected::new()
	}
}

#[cfg(test)]
pub(self) mod test {
	use super::*;
	use tokio::io::{AsyncReadExt, AsyncWriteExt};

	#[tokio::test]
	async fn should_send_and_receive_data() {
		let (mut alice, mut bob) = Connection::new();
		const HI_BOB: &[u8] = b"Hi Bob!";
		const HI_ALICE: &[u8] = b"Hi Alice!";

		let mut bob_receive_buffer = [0u8; HI_BOB.len()];
		let mut alice_receive_buffer = [0u8; HI_ALICE.len()];

		alice.write_all(HI_BOB).await.expect("Failed to send greeting to bob.");
		bob.read_exact(&mut bob_receive_buffer)
			.await
			.expect("Failed to receive greeting to bob.");

		bob.write_all(HI_ALICE)
			.await
			.expect("Failed to send greeting to alice.");
		alice
			.read_exact(&mut alice_receive_buffer)
			.await
			.expect("Failed to receive greeting to alice");

		assert_eq!(bob_receive_buffer, HI_BOB, "Bob didn't receive the correct greeting.");
		assert_eq!(
			alice_receive_buffer, HI_ALICE,
			"Alice didn't receive the correct greeting."
		);
	}

	#[tokio::test]
	async fn should_fail_with_dropped_connection() {
		let (mut alice, bob) = Connection::new();
		drop(bob);

		alice
			.write_all(b"Hi Bob!")
			.await
			.expect_err("Should have failed to send to a closed connection.");
		alice
			.read_u8()
			.await
			.expect_err("Should have failed to read from a closed connection.");
	}

	#[track_caller]
	pub async fn ensure_connection_works(mut client: Connection, mut server: Connection) {
		client.write_u8(42).await.expect("Failed to send byte");
		assert_eq!(
			42,
			server.read_u8().await.expect("Failed to receive byte."),
			"Received incorrect byte"
		);
	}
}
