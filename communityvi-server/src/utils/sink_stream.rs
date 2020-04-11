use futures::task::Context;
use futures::{Sink, Stream};
use std::pin::Pin;
use std::task::Poll;

/// Combination of Sink and Stream in one type
pub struct SinkStream<SinkType, StreamType> {
	pub sink: SinkType,
	pub stream: StreamType,
}

impl<SinkType, StreamType> SinkStream<SinkType, StreamType> {
	pub fn new(sink: SinkType, stream: StreamType) -> Self {
		Self { sink, stream }
	}
}

impl<SinkType: Unpin, StreamType: Stream + Unpin> Stream for SinkStream<SinkType, StreamType> {
	type Item = StreamType::Item;

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
		Pin::new(&mut self.as_mut().stream).poll_next(context)
	}
}

impl<Item, SinkType: Sink<Item> + Unpin, StreamType: Unpin> Sink<Item> for SinkStream<SinkType, StreamType> {
	type Error = SinkType::Error;

	fn poll_ready(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Result<(), Self::Error>> {
		Pin::new(&mut self.as_mut().sink).poll_ready(context)
	}

	fn start_send(mut self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
		Pin::new(&mut self.as_mut().sink).start_send(item)
	}

	fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Result<(), Self::Error>> {
		Pin::new(&mut self.as_mut().sink).poll_flush(context)
	}

	fn poll_close(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Result<(), Self::Error>> {
		Pin::new(&mut self.as_mut().sink).poll_close(context)
	}
}
