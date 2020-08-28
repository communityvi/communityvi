use futures::task::{Context, Poll};
use futures::{Stream, StreamExt};
use pin_project::pin_project;
use std::any::type_name;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::{broadcast, Notify};
use tokio::time::{interval_at, timeout};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct TimeSource {
	test_timesources: Option<Arc<TestTimeSources>>,
}

pub struct TestTimeSources {
	named_time_sources: parking_lot::Mutex<BTreeMap<&'static str, Arc<TestTimeSource>>>,
}

impl Default for TestTimeSources {
	fn default() -> Self {
		Self {
			named_time_sources: Default::default(),
		}
	}
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

impl TestTimeSources {
	fn interval_at(&self, name: &'static str, start: Duration, period: Duration) -> TestInterval {
		let mut time_sources = self.named_time_sources.lock();
		let time_source = time_sources.entry(name).or_default();
		let interval = TestInterval {
			current_time: Default::default(),
			next_deadline: start,
			period,
			receiver: time_source.time_sender.subscribe(),
		};

		time_source.notification.notify();

		interval
	}

	fn timeout<ValueFuture: Future>(&self, name: &'static str, duration: Duration, future: ValueFuture) -> TestTimeout<ValueFuture> {
		let mut time_sources = self.named_time_sources.lock();
		let time_source = time_sources.entry(name).or_default();
		let timeout = TestTimeout {
			future,
			current_time: Default::default(),
			deadline: duration,
			receiver: time_source.time_sender.subscribe(),
		};

		time_source.notification.notify();

		timeout
	}

	fn advance_time(&self, name: &'static str, by_duration: Duration) {
		let time_sources = self.named_time_sources.lock();
		let time_source = time_sources.get(name).expect("No time sender of this name");
		let _ = time_source.time_sender.send(by_duration); // ignore error so this works even without anyone waiting
	}

	async fn wait_for_time_request(&self, name: &'static str) {
		let time_source = {
			// subscope so the MutexGuard isn't held across an await point
			let mut time_sources = self.named_time_sources.lock();
			time_sources.entry(name).or_default().clone()
		};

		time_source.notification.notified().await;
	}
}

impl TimeSource {
	pub fn test() -> Self {
		Self {
			test_timesources: Some(Default::default()),
		}
	}

	pub fn interval_at(&self, name: &'static str, start: Duration, period: Duration) -> Interval {
		match &self.test_timesources {
			None => Interval::Tokio(interval_at(tokio::time::Instant::now() + start, period)),
			Some(test_time_source) => Interval::Test(test_time_source.interval_at(name, start, period)),
		}
	}

	pub fn timeout<ValueFuture: Future>(&self, name: &'static str, duration: Duration, future: ValueFuture) -> Timeout<ValueFuture> {
		match &self.test_timesources {
			None => Timeout::Tokio(timeout(duration, future)),
			Some(test_time_source) => Timeout::Test(test_time_source.timeout(name, duration, future)),
		}
	}

	pub fn advance_time(&self, name: &'static str, by_duration: Duration) {
		let _ = self.test_timesources
			.as_ref()
			.expect("Can only be called in test mode.")
			.advance_time(name, by_duration);
	}

	pub async fn wait_for_time_request(&self, name: &'static str) {
		match &self.test_timesources {
			None => (),
			Some(test_time_source) => test_time_source.wait_for_time_request(name).await,
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
	use std::fmt::Debug;

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
		let interval = time_source.interval_at("irrelevant", start, period);

		matches!(interval, Interval::Tokio(_));
		interval
	}

	#[tokio::test]
	async fn test_interval_should_only_trigger_when_advanced_to_its_start_time() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(1337);
		let period = Duration::from_secs(42);
		let mut interval = time_source.interval_at("test", start, period);
		matches!(interval, Interval::Test(_));

		let mut start_future = interval.tick();
		let mut pinned_future = unsafe { Pin::new_unchecked(&mut start_future) };
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time("test", Duration::from_secs(42));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time("test", Duration::from_secs(1337 - 42));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Ready(()));
	}

	#[tokio::test]
	async fn test_interval_should_trigger_when_advanced_past_its_start_time() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(1337);
		let period = Duration::from_secs(42);
		let mut interval = time_source.interval_at("interval", start, period);
		matches!(interval, Interval::Test(_));

		let mut start_future = interval.tick();
		let mut pinned_future = unsafe { Pin::new_unchecked(&mut start_future) };
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time("interval", Duration::from_secs(42));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Pending);

		time_source.advance_time("interval", Duration::from_secs(1337));
		assert_eq!(poll!(pinned_future.as_mut()), Poll::Ready(()));
	}

	#[tokio::test]
	async fn test_interval_should_trigger_after_period() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(0);
		let period = Duration::from_secs(42);
		let mut interval = time_source.interval_at("nirvana", start, period);
		matches!(interval, Interval::Test(_));

		interval.tick().await;

		{
			let mut first_period_future = interval.tick();
			let mut pinned_first_period_future = unsafe { Pin::new_unchecked(&mut first_period_future) };
			assert_eq!(poll!(pinned_first_period_future.as_mut()), Poll::Pending);

			time_source.advance_time("nirvana", Duration::from_secs(1));
			assert_eq!(poll!(pinned_first_period_future.as_mut()), Poll::Pending);

			time_source.advance_time("nirvana", Duration::from_secs(41));
			assert_eq!(poll!(pinned_first_period_future.as_mut()), Poll::Ready(()));
		}

		{
			let mut second_period_future = interval.tick();
			let mut pinned_second_period_future = unsafe { Pin::new_unchecked(&mut second_period_future) };
			assert_eq!(poll!(pinned_second_period_future.as_mut()), Poll::Pending);

			time_source.advance_time("nirvana", Duration::from_secs(10));
			assert_eq!(poll!(pinned_second_period_future.as_mut()), Poll::Pending);

			time_source.advance_time("nirvana", Duration::from_secs(40));
			assert_eq!(poll!(pinned_second_period_future.as_mut()), Poll::Ready(()));
		}
	}

	#[tokio::test]
	async fn test_time_source_should_advance_time_with_cloned_objects() {
		let original_time_source = TimeSource::test();
		let mut interval = original_time_source.interval_at("dolly", Duration::from_millis(1), Duration::from_millis(1));
		matches!(interval, Interval::Test(_));

		let cloned_time_source = original_time_source.clone();
		cloned_time_source.advance_time("dolly", Duration::from_millis(1));
		assert_poll(Poll::Ready(()), interval.tick()).await;
	}

	#[tokio::test]
	async fn test_interval_should_trigger_multiple_times_after_advancing_multiple_period_lengths() {
		let time_source = TimeSource::test();

		let start = Duration::from_secs(10);
		let period = Duration::from_secs(100);
		let mut interval = time_source.interval_at("multiple", start, period);
		matches!(interval, Interval::Test(_));

		time_source.advance_time("multiple", Duration::from_secs(210));

		assert_poll(Poll::Ready(()), interval.tick()).await;
		assert_poll(Poll::Ready(()), interval.tick()).await;
		assert_poll(Poll::Ready(()), interval.tick()).await;
		assert_poll(Poll::Pending, interval.tick()).await;
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_timeout_that_elapses() {
		let time_source = TimeSource::default();

		let timeout = time_source.timeout("irrelevant", Duration::from_millis(1), futures::future::pending::<u8>());
		assert_eq!(timeout.await, Err(()));
	}

	#[tokio::test]
	async fn time_source_should_create_tokio_timeout_that_succeeds() {
		let time_source = TimeSource::default();

		let timeout = time_source.timeout("irrelevant", Duration::from_millis(1), futures::future::ready(42));
		assert_eq!(timeout.await, Ok(42));
	}

	#[tokio::test]
	async fn test_timeout_should_not_time_out_too_early() {
		let time_source = TimeSource::test();

		let timeout = time_source.timeout("early bird", Duration::from_millis(1337), futures::future::ready(42));
		assert_eq!(timeout.await, Ok(42));

		let timeout = time_source.timeout("early bird", Duration::from_millis(1337), futures::future::ready(42));
		time_source.advance_time("early bird", Duration::from_millis(42));
		assert_eq!(timeout.await, Ok(42));
	}

	#[tokio::test]
	async fn test_timeout_should_time_out() {
		let time_source = TimeSource::test();

		let timeout = time_source.timeout("tick tock", Duration::from_millis(1337), futures::future::ready(42));
		time_source.advance_time("tick tock", Duration::from_millis(1337));
		assert_eq!(timeout.await, Err(()));

		let timeout = time_source.timeout("tick tock", Duration::from_millis(1), futures::future::pending::<u8>());
		time_source.advance_time("tick tock", Duration::from_millis(1));
		assert_eq!(timeout.await, Err(()));
	}

	#[tokio::test]
	async fn test_timeout_should_trigger_time_request() {
		const TIMEOUT_NAME: &str = "timeout";
		let time_source = TimeSource::test();

		assert_poll(Poll::Pending, time_source.wait_for_time_request(TIMEOUT_NAME)).await;

		let wait_before = time_source.wait_for_time_request(TIMEOUT_NAME);
		time_source.timeout(TIMEOUT_NAME, Duration::from_millis(1337), futures::future::ready(())).await.expect("Timeout failed");
		assert_poll(Poll::Ready(()), wait_before).await;

		time_source.timeout(TIMEOUT_NAME, Duration::from_millis(1337), futures::future::ready(())).await.expect("Timeout failed");
		assert_poll(Poll::Ready(()), time_source.wait_for_time_request(TIMEOUT_NAME)).await;
	}

	#[tokio::test]
	async fn test_timeout_with_different_name_should_not_trigger_time_request() {
		const TIMEOUT_NAME: &str = "timeout";
		const WAIT_NAME: &str = "infinity";
		let time_source = TimeSource::test();

		assert_poll(Poll::Pending, time_source.wait_for_time_request(WAIT_NAME)).await;

		let wait_before = time_source.wait_for_time_request(WAIT_NAME);
		time_source.timeout(TIMEOUT_NAME, Duration::from_millis(1337), futures::future::ready(())).await.expect("Timeout failed");
		assert_poll(Poll::Pending, wait_before).await;

		time_source.timeout(TIMEOUT_NAME, Duration::from_millis(1337), futures::future::ready(())).await.expect("Timeout failed");
		assert_poll(Poll::Pending, time_source.wait_for_time_request(WAIT_NAME)).await;
	}

	#[tokio::test]
	async fn test_interval_with_different_name_should_not_trigger_time_request() {
		const INTERVAL_NAME: &str = "interval";
		const WAIT_NAME: &str = "infinity";
		let time_source = TimeSource::test();

		assert_poll(Poll::Pending, time_source.wait_for_time_request(WAIT_NAME)).await;

		let wait_before = time_source.wait_for_time_request(WAIT_NAME);
		let mut interval = time_source.interval_at(INTERVAL_NAME, Duration::from_millis(0), Duration::from_millis(1));
		interval.tick().await;
		assert_poll(Poll::Pending, wait_before).await;

		let mut interval = time_source.interval_at(INTERVAL_NAME, Duration::from_millis(0), Duration::from_millis(1));
		interval.tick().await;
		assert_poll(Poll::Pending, time_source.wait_for_time_request(WAIT_NAME)).await;
	}

	#[must_use = "async functions must be awaited."]
	async fn assert_poll<OutputType: Debug + PartialEq>(expected: Poll<OutputType>, mut future: impl Future<Output = OutputType>) {
		let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
		assert_eq!(expected, poll!(pinned.as_mut()));
	}
}
