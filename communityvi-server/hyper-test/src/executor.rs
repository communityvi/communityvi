use std::future::Future;

#[derive(Clone, Copy)]
pub struct Executor;

impl<T> hyper::rt::Executor<T> for Executor
where
	T: Future + Send + 'static,
	T::Output: Send,
{
	fn execute(&self, future: T) {
		tokio::spawn(future);
	}
}
