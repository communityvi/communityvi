use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::task::JoinHandle;

/// Wrapper around [`tokio::task::JoinHandle`] that aborts the task once dropped.
pub struct AbortingJoinHandle<T>(JoinHandle<T>);

impl<T> Future for AbortingJoinHandle<T> {
	type Output = <JoinHandle<T> as Future>::Output;

	fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.get_mut().0).poll(context)
	}
}

impl<T> From<JoinHandle<T>> for AbortingJoinHandle<T> {
	fn from(join_handle: JoinHandle<T>) -> Self {
		Self(join_handle)
	}
}

impl<T> Drop for AbortingJoinHandle<T> {
	fn drop(&mut self) {
		self.0.abort();
	}
}
