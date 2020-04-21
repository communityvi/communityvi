use futures::{Sink, SinkExt, Stream, StreamExt};
use std::fmt::Debug;

pub struct TestClient<ClientSink, ClientStream> {
	sink: ClientSink,
	stream: ClientStream,
}

impl<ClientSink, ClientStream> TestClient<ClientSink, ClientStream> {
	pub fn new(sink: ClientSink, stream: ClientStream) -> Self {
		Self { sink, stream }
	}

	pub fn split(self) -> (ClientSink, ClientStream) {
		let TestClient { sink, stream } = self;
		(sink, stream)
	}

	pub async fn send<Request, SinkError>(&mut self, request: Request)
	where
		ClientSink: Sink<Request, Error = SinkError> + Unpin,
		SinkError: Debug,
	{
		self.sink
			.send(request)
			.await
			.expect("Failed to send message via TestClient.");
	}

	pub async fn receive<Response>(&mut self) -> Response
	where
		ClientStream: Stream<Item = Response> + Unpin,
	{
		self.stream
			.next()
			.await
			.expect("Failed to receive message via TestClient")
	}
}
