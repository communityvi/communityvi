use futures::task::{Context, Poll};
use pin_project::pin_project;
use std::future::Future;
use tokio::macros::support::Pin;

/// Creates a future that runs two futures simultaneously but finishes once the
/// first has finished, essentially racing them against each other.
pub fn select_first_future<FutureA, FutureB, OutputType>(
	future_a: FutureA,
	future_b: FutureB,
) -> SelectFirstFuture<FutureA, FutureB>
where
	FutureA: Future<Output = OutputType>,
	FutureB: Future<Output = OutputType>,
{
	SelectFirstFuture { future_a, future_b }
}

#[pin_project]
pub struct SelectFirstFuture<FutureA, FutureB> {
	#[pin]
	future_a: FutureA,
	#[pin]
	future_b: FutureB,
}

impl<FutureA, FutureB, OutputType> Future for SelectFirstFuture<FutureA, FutureB>
where
	FutureA: Future<Output = OutputType>,
	FutureB: Future<Output = OutputType>,
{
	type Output = OutputType;

	fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
		let projected = self.project();
		match projected.future_a.poll(context) {
			Poll::Ready(output) => Poll::Ready(output),
			Poll::Pending => projected.future_b.poll(context),
		}
	}
}
