use futures_util::{Stream, StreamExt};
use pin_project::pin_project;
use std::any::type_name;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{Notify, broadcast};
use tokio::time::{interval_at, timeout};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone, Default)]
pub struct TimeSource {
	test_timesource: Option<Arc<TestTimeSource>>,
}

pub struct TestTimeSource {
	time_sender: broadcast::Sender<Duration>,
	notification: Notify,
}

impl Default for TestTimeSource {
	fn default() -> Self {
		Self {
			time_sender: broadcast::channel(16).0,
			notification: Default::default(),
		}
	}
}

impl TestTimeSource {
	fn interval_at(&self, start: Duration, period: Duration) -> TestInterval {
		let interval = TestInterval {
			current_time: Default::default(),
			next_deadline: start,
			period,
			receiver: Box::pin(BroadcastStream::new(self.time_sender.subscribe())),
		};

		// FIXME: Check if this use of tokio::sync::Notify is correct!
		self.notification.notify_one();

		interval
	}

	fn timeout<ValueFuture: Future>(&self, duration: Duration, future: ValueFuture) -> TestTimeout<ValueFuture> {
		let timeout = TestTimeout {
			future,
			current_time: Default::default(),
			deadline: duration,
			receiver: BroadcastStream::new(self.time_sender.subscribe()),
		};

		// FIXME: Check if this use of tokio::sync::Notify is correct!
		self.notification.notify_one();

		timeout
	}

	#[cfg(test)]
	fn advance_time(&self, by_duration: Duration) {
		let _ = self.time_sender.send(by_duration); // ignore error so this works even without anyone waiting
	}

	#[cfg(test)]
	async fn wait_for_time_request(&self) {
		self.notification.notified().await;
	}
}

impl TimeSource {
	#[cfg(test)]
	pub fn test() -> Self {
		Self {
			test_timesource: Some(Default::default()),
		}
	}

	pub fn interval_at(&self, start: Duration, period: Duration) -> Interval {
		match &self.test_timesource {
			None => Interval::Tokio(interval_at(tokio::time::Instant::now() + start, period)),
			Some(test_time_source) => Interval::Test(test_time_source.interval_at(start, period)),
		}
	}

	pub fn timeout<ValueFuture: Future>(&self, duration: Duration, future: ValueFuture) -> Timeout<ValueFuture> {
		match &self.test_timesource {
			None => Timeout::Tokio(timeout(duration, future)),
			Some(test_time_source) => Timeout::Test(test_time_source.timeout(duration, future)),
		}
	}

	#[cfg(test)]
	pub fn advance_time(&self, by_duration: Duration) {
		self.test_timesource
			.as_ref()
			.expect("Can only be called in test mode.")
			.advance_time(by_duration);
	}

	#[cfg(test)]
	pub async fn wait_for_time_request(&self) {
		match &self.test_timesource {
			None => (),
			Some(test_time_source) => test_time_source.wait_for_time_request().await,
		}
	}
}

pub enum Interval {
	Tokio(tokio::time::Interval),
	Test(TestInterval),
}

impl Interval {
	pub async fn tick(&mut self) {
		match self {
			Interval::Tokio(interval) => {
				interval.tick().await;
			}
			Interval::Test(interval) => interval
				.next()
				.await
				.unwrap_or_else(|| panic!("{} dropped prematurely.", type_name::<TimeSource>())),
		}
	}
}

pub struct TestInterval {
	current_time: Duration,
	next_deadline: Duration,
	period: Duration,
	receiver: Pin<
		Box<
			dyn Stream<Item = Result<Duration, tokio_stream::wrappers::errors::BroadcastStreamRecvError>>
				+ Send
				+ 'static,
		>,
	>,
}

impl Stream for TestInterval {
	type Item = ();

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
		let receive_poll = self.as_mut().receiver.poll_next_unpin(context);
		match receive_poll {
			Poll::Ready(Some(time_delta)) => {
				self.as_mut().current_time += time_delta.expect("Failed to receive current time.");
			}
			Poll::Ready(None) => return Poll::Ready(None),
			Poll::Pending => {}
		}

		if self.current_time >= self.next_deadline {
			let period = self.period;
			self.next_deadline += period;
			return Poll::Ready(Some(()));
		}

		Poll::Pending
	}
}

#[pin_project(project = ProjectedTimeout)]
pub enum Timeout<ValueFuture> {
	Tokio(#[pin] tokio::time::Timeout<ValueFuture>),
	Test(#[pin] TestTimeout<ValueFuture>),
}

impl<ValueFuture: Future> Future for Timeout<ValueFuture> {
	type Output = Result<ValueFuture::Output, ()>;

	fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
		let this = self.project();
		match this {
			ProjectedTimeout::Tokio(timeout) => match timeout.poll(context) {
				Poll::Ready(Ok(output)) => Poll::Ready(Ok(output)),
				Poll::Ready(Err(_)) => Poll::Ready(Err(())),
				Poll::Pending => Poll::Pending,
			},
			ProjectedTimeout::Test(timeout) => timeout.poll(context),
		}
	}
}

#[pin_project]
pub struct TestTimeout<ValueFuture> {
	#[pin]
	future: ValueFuture,
	current_time: Duration,
	deadline: Duration,
	#[pin]
	receiver: BroadcastStream<Duration>,
}

impl<ValueFuture: Future> Future for TestTimeout<ValueFuture> {
	type Output = Result<ValueFuture::Output, ()>;

	fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
		let this = self.project();
		let receive_poll = this.receiver.poll_next(context);
		match receive_poll {
			Poll::Ready(Some(time_delta)) => *this.current_time += time_delta.expect("Failed to receive current time."),
			Poll::Ready(None) => return Poll::Ready(Err(())),
			Poll::Pending => {}
		}

		if this.current_time >= this.deadline {
			return Poll::Ready(Err(()));
		}

		let value_poll = this.future.poll(context);
		match value_poll {
			Poll::Ready(value) => return Poll::Ready(Ok(value)),
			Poll::Pending => {}
		}

		Poll::Pending
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use futures_util::{future, poll};
	use std::fmt::Debug;
	use tokio::time::timeout;

	#[tokio::test]
	async fn time_source_should_create_tokio_interval_with_correct_short_period() {
		let mut interval =
			create_tokio_based_interval_via_time_source(Duration::from_millis(0), Duration::from_millis(1));

		timeout(Duration::from_millis(100), interval.tick())
			.await
			.expect("Incorrect start time");
		timeout(Duration::from_millis(100), interval.tick())
			.await
			.expect("Incorrect period");
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_interval_with_long_period() {
		let mut interval =
			create_tokio_based_interval_via_time_source(Duration::from_millis(0), Duration::from_secs(1));

		timeout(Duration::from_millis(500), interval.tick())
			.await
			.expect("Incorrect start time");
		timeout(Duration::from_millis(10), interval.tick())
			.await
			.expect_err("Incorrect period");
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_interval_with_long_start_time() {
		let mut interval = create_tokio_based_interval_via_time_source(Duration::from_secs(1), Duration::from_secs(1));

		timeout(Duration::from_millis(500), interval.tick())
			.await
			.expect_err("Incorrect start time");
	}

	fn create_tokio_based_interval_via_time_source(start: Duration, period: Duration) -> Interval {
		let time_source = TimeSource::default();
		let interval = time_source.interval_at(start, period);

		matches!(interval, Interval::Tokio(_));
		interval
	}

	#[tokio::test]
	async fn test_interval_should_only_trigger_when_advanced_to_its_start_time() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(1337);
		let period = Duration::from_secs(42);
		let mut interval = time_source.interval_at(start, period);
		matches!(interval, Interval::Test(_));

		let mut start_future = interval.tick();
		let mut pinned_future = unsafe { Pin::new_unchecked(&mut start_future) };
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time(Duration::from_secs(42));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time(Duration::from_secs(1337 - 42));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Ready(()));
	}

	#[tokio::test]
	async fn test_interval_should_trigger_when_advanced_past_its_start_time() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(1337);
		let period = Duration::from_secs(42);
		let mut interval = time_source.interval_at(start, period);
		matches!(interval, Interval::Test(_));

		let mut start_future = interval.tick();
		let mut pinned_future = unsafe { Pin::new_unchecked(&mut start_future) };
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time(Duration::from_secs(42));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time(Duration::from_secs(1337));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Ready(()));
	}

	#[tokio::test]
	async fn test_interval_should_trigger_after_period() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(0);
		let period = Duration::from_secs(42);
		let mut interval = time_source.interval_at(start, period);
		matches!(interval, Interval::Test(_));

		interval.tick().await;

		{
			let mut first_period_future = interval.tick();
			let mut pinned_first_period_future = unsafe { Pin::new_unchecked(&mut first_period_future) };
			assert_eq!(poll!(pinned_first_period_future.as_mut()), Poll::Pending);

			time_source.advance_time(Duration::from_secs(1));
			assert_eq!(poll!(pinned_first_period_future.as_mut()), Poll::Pending);

			time_source.advance_time(Duration::from_secs(41));
			assert_eq!(poll!(pinned_first_period_future.as_mut()), Poll::Ready(()));
		}

		{
			let mut second_period_future = interval.tick();
			let mut pinned_second_period_future = unsafe { Pin::new_unchecked(&mut second_period_future) };
			assert_eq!(poll!(pinned_second_period_future.as_mut()), Poll::Pending);

			time_source.advance_time(Duration::from_secs(10));
			assert_eq!(poll!(pinned_second_period_future.as_mut()), Poll::Pending);

			time_source.advance_time(Duration::from_secs(40));
			assert_eq!(poll!(pinned_second_period_future.as_mut()), Poll::Ready(()));
		}
	}

	#[tokio::test]
	async fn test_time_source_should_advance_time_with_cloned_objects() {
		let original_time_source = TimeSource::test();
		let mut interval = original_time_source.interval_at(Duration::from_millis(1), Duration::from_millis(1));
		matches!(interval, Interval::Test(_));

		let cloned_time_source = original_time_source.clone();
		cloned_time_source.advance_time(Duration::from_millis(1));
		assert_poll(Poll::Ready(()), interval.tick()).await;
	}

	#[tokio::test]
	async fn test_interval_should_trigger_multiple_times_after_advancing_multiple_period_lengths() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(10);
		let period = Duration::from_secs(100);
		let mut interval = time_source.interval_at(start, period);
		matches!(interval, Interval::Test(_));

		time_source.advance_time(Duration::from_secs(210));

		assert_poll(Poll::Ready(()), interval.tick()).await;
		assert_poll(Poll::Ready(()), interval.tick()).await;
		assert_poll(Poll::Ready(()), interval.tick()).await;
		assert_poll(Poll::Pending, interval.tick()).await;
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_timeout_that_elapses() {
		let time_source = TimeSource::default();

		let timeout = time_source.timeout(Duration::from_millis(1), future::pending::<u8>());
		assert_eq!(timeout.await, Err(()));
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_timeout_that_succeeds() {
		let time_source = TimeSource::default();

		let timeout = time_source.timeout(Duration::from_millis(1), future::ready(42));
		assert_eq!(timeout.await, Ok(42));
	}

	#[tokio::test]
	async fn test_timeout_should_not_time_out_too_early() {
		let time_source = TimeSource::test();

		let timeout = time_source.timeout(Duration::from_millis(1337), future::ready(42));
		assert_eq!(timeout.await, Ok(42));

		let timeout = time_source.timeout(Duration::from_millis(1337), future::ready(42));
		time_source.advance_time(Duration::from_millis(42));
		assert_eq!(timeout.await, Ok(42));
	}

	#[tokio::test]
	async fn test_timeout_should_time_out() {
		let time_source = TimeSource::test();

		let timeout = time_source.timeout(Duration::from_millis(1337), future::ready(42));
		time_source.advance_time(Duration::from_millis(1337));
		assert_eq!(timeout.await, Err(()));

		let timeout = time_source.timeout(Duration::from_millis(1), future::pending::<u8>());
		time_source.advance_time(Duration::from_millis(1));
		assert_eq!(timeout.await, Err(()));
	}

	#[tokio::test]
	async fn test_timeout_should_trigger_time_request() {
		let time_source = TimeSource::test();

		assert_poll(Poll::Pending, time_source.wait_for_time_request()).await;

		let wait_before = time_source.wait_for_time_request();
		time_source
			.timeout(Duration::from_millis(1337), future::ready(()))
			.await
			.expect("Timeout failed");
		assert_poll(Poll::Ready(()), wait_before).await;

		time_source
			.timeout(Duration::from_millis(1337), future::ready(()))
			.await
			.expect("Timeout failed");
		assert_poll(Poll::Ready(()), time_source.wait_for_time_request()).await;
	}

	async fn assert_poll<OutputType: Debug + PartialEq>(
		expected: Poll<OutputType>,
		mut future: impl Future<Output = OutputType>,
	) {
		let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
		assert_eq!(expected, poll!(pinned.as_mut()));
	}
}
