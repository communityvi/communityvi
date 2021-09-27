use crate::message::WebSocketMessage;
use futures::Sink;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug, Default)]
pub struct FakeMessageSender {}

impl Sink<WebSocketMessage> for FakeMessageSender {
	type Error = anyhow::Error;

	fn poll_ready(self: Pin<&mut Self>, _context: &mut Context) -> Poll<anyhow::Result<()>> {
		Poll::Ready(Ok(()))
	}

	fn start_send(self: Pin<&mut Self>, _item: WebSocketMessage) -> anyhow::Result<()> {
		Ok(())
	}

	fn poll_flush(self: Pin<&mut Self>, _context: &mut Context) -> Poll<anyhow::Result<()>> {
		Poll::Ready(Ok(()))
	}

	fn poll_close(self: Pin<&mut Self>, _context: &mut Context) -> Poll<anyhow::Result<()>> {
		Poll::Ready(Ok(()))
	}
}
