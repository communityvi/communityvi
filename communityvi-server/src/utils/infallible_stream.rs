use futures::task::Context;
use futures::Stream;
use std::fmt::Display;
use tokio::macros::support::{Pin, Poll};

/// Takes a `TryStream` and creates a new stream from it that ends as soon as an error
/// occurs in the underlying stream.
pub struct InfallibleStream<FallibleStream> {
	stream: FallibleStream,
}

impl<FallibleStream, ItemType, ErrorType> From<FallibleStream> for InfallibleStream<FallibleStream>
where
	FallibleStream: Stream<Item = Result<ItemType, ErrorType>>,
	ErrorType: Display,
{
	fn from(stream: FallibleStream) -> Self {
		Self { stream }
	}
}

impl<FallibleStream, ItemType, ErrorType> Stream for InfallibleStream<FallibleStream>
where
	FallibleStream: Stream<Item = Result<ItemType, ErrorType>> + Unpin,
	ErrorType: Display,
{
	type Item = ItemType;

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
		match Pin::new(&mut self.as_mut().stream).poll_next(context) {
			Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(item)),
			Poll::Ready(Some(Err(error))) => {
				log::error!("{}", error);
				Poll::Ready(None)
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use futures::{SinkExt, StreamExt};

	#[tokio::test]
	async fn should_stop_streaming_when_an_error_occurs() {
		let (mut sender, receiver) = futures::channel::mpsc::channel(100);
		let _ = sender.send(Ok(1)).await;
		let _ = sender.send(Ok(2)).await;
		let _ = sender.send(Err("error")).await;

		let mut infallible = InfallibleStream::from(receiver);

		let first = infallible.next().await;
		assert_eq!(Some(1), first);
		let second = infallible.next().await;
		assert_eq!(Some(2), second);
		let end = infallible.next().await;
		assert_eq!(None, end);
	}
}
