use futures::task::{Context, Poll};
use futures::{Stream, StreamExt};
use pin_project::pin_project;
use std::any::type_name;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{interval_at, timeout};

#[derive(Default)]
pub struct TimeSource {
	test_channel: Option<broadcast::Sender<Duration>>,
}

impl TimeSource {
	pub fn test() -> Self {
		Self {
			test_channel: Some(broadcast::channel(16).0),
		}
	}

	pub fn interval_at(&self, start: Duration, period: Duration) -> Interval {
		match &self.test_channel {
			None => Interval::Tokio(interval_at(tokio::time::Instant::now() + start, period)),
			Some(sender) => Interval::Test(TestInterval {
				current_time: Default::default(),
				next_deadline: start,
				period,
				receiver: sender.subscribe(),
			}),
		}
	}

	pub fn timeout<ValueFuture: Future>(&self, duration: Duration, future: ValueFuture) -> Timeout<ValueFuture> {
		match &self.test_channel {
			None => Timeout::Tokio(timeout(duration, future)),
			Some(sender) => Timeout::Test(TestTimeout {
				future,
				current_time: Default::default(),
				deadline: duration,
				receiver: sender.subscribe(),
			}),
		}
	}

	pub fn advance_time(&self, by_duration: Duration) {
		self.test_channel
			.as_ref()
			.expect("Can only be called in test mode.")
			.send(by_duration)
			.expect("Failed to advance time");
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
		};
	}
}

pub struct TestInterval {
	current_time: Duration,
	next_deadline: Duration,
	period: Duration,
	receiver: broadcast::Receiver<Duration>,
}

impl Stream for TestInterval {
	type Item = ();

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
		let receive_poll = self.as_mut().receiver.poll_next_unpin(context);
		match receive_poll {
			Poll::Ready(Some(time_delta)) => {
				self.as_mut().current_time += time_delta.expect("Failed to receive current time.")
			}
			Poll::Ready(None) => return Poll::Ready(None),
			Poll::Pending => {}
		};

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
	receiver: broadcast::Receiver<Duration>,
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
		};

		Poll::Pending
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use futures::poll;
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
	async fn test_interval_should_trigger_multiple_times_after_advancing_multiple_period_lengths() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(10);
		let period = Duration::from_secs(100);
		let mut interval = time_source.interval_at(start, period);
		matches!(interval, Interval::Test(_));

		time_source.advance_time(Duration::from_secs(210));

		{
			let mut start_future = interval.tick();
			let mut pinned_start_future = unsafe { Pin::new_unchecked(&mut start_future) };
			assert_eq!(poll!(pinned_start_future.as_mut()), Poll::Ready(()));
		}
		{
			let mut first_period_future = interval.tick();
			let mut pinned_first_period_future = unsafe { Pin::new_unchecked(&mut first_period_future) };
			assert_eq!(poll!(pinned_first_period_future.as_mut()), Poll::Ready(()));
		}
		{
			let mut second_period_future = interval.tick();
			let mut pinned_second_period_future = unsafe { Pin::new_unchecked(&mut second_period_future) };
			assert_eq!(poll!(pinned_second_period_future.as_mut()), Poll::Ready(()));
		}
		{
			let mut third_period_future = interval.tick();
			let mut pinned_third_period_future = unsafe { Pin::new_unchecked(&mut third_period_future) };
			assert_eq!(poll!(pinned_third_period_future.as_mut()), Poll::Pending);
		}
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_timeout_that_elapses() {
		let time_source = TimeSource::default();

		let timeout = time_source.timeout(Duration::from_millis(1), futures::future::pending::<u8>());
		assert_eq!(timeout.await, Err(()));
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_timeout_that_succeeds() {
		let time_source = TimeSource::default();

		let timeout = time_source.timeout(Duration::from_millis(1), futures::future::ready(42));
		assert_eq!(timeout.await, Ok(42));
	}

	#[tokio::test]
	async fn test_timeout_should_not_time_out_too_early() {
		let time_source = TimeSource::test();

		let timeout = time_source.timeout(Duration::from_millis(1337), futures::future::ready(42));
		assert_eq!(timeout.await, Ok(42));

		let timeout = time_source.timeout(Duration::from_millis(1337), futures::future::ready(42));
		time_source.advance_time(Duration::from_millis(42));
		assert_eq!(timeout.await, Ok(42));
	}

	#[tokio::test]
	async fn test_timeout_should_time_out() {
		let time_source = TimeSource::test();

		let timeout = time_source.timeout(Duration::from_millis(1337), futures::future::ready(42));
		time_source.advance_time(Duration::from_millis(1337));
		assert_eq!(timeout.await, Err(()));

		let timeout = time_source.timeout(Duration::from_millis(1), futures::future::pending::<u8>());
		time_source.advance_time(Duration::from_millis(1));
		assert_eq!(timeout.await, Err(()));
	}
}
