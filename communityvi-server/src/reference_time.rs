use chrono::{DateTime, Utc};
use js_int::UInt;
use quanta::Clock;

#[derive(Clone)]
pub struct ReferenceTimer {
	start: quanta::Instant,
	clock: Clock,
	/// Offset from UNIX epoch
	start_offset: std::time::Duration,
}

impl ReferenceTimer {
	pub fn reference_time(&self) -> std::time::Duration {
		self.clock.now().duration_since(self.start) + self.start_offset
	}

	pub fn reference_time_milliseconds(&self) -> UInt {
		#[allow(clippy::cast_possible_truncation)]
		let milliseconds = self.reference_time().as_millis() as u64;
		UInt::try_from(milliseconds)
			.expect("More milliseconds than can be represented by IEEE 754 doubles. This shouldn't happen unless the server was running for more than about 285000 years.")
	}

	#[allow(unused)]
	pub fn with_clock(mut self, clock: Clock) -> Self {
		self.start = clock.now();
		self.clock = clock;
		self
	}

	#[allow(unused)]
	pub fn with_start_time(mut self, start_time: DateTime<Utc>) -> Self {
		self.start_offset = (start_time - DateTime::UNIX_EPOCH)
			.to_std()
			.expect("Time since UNIX epoch was negative or out of range");
		self
	}
}

impl Default for ReferenceTimer {
	fn default() -> Self {
		let clock = Clock::default();
		let start_offset = (Utc::now() - DateTime::UNIX_EPOCH)
			.to_std()
			.expect("Time since UNIX epoch was negative or out of range");
		Self {
			start: clock.now(),
			clock,
			start_offset,
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use js_int::uint;
	use std::time::Duration;

	#[test]
	fn should_time_the_passage_of_reference_time() {
		let (clock, clock_mock) = Clock::mock();
		let reference_timer = ReferenceTimer::default().with_clock(clock);

		let initial_time = reference_timer.reference_time();
		clock_mock.increment(Duration::from_millis(1));
		let final_time = reference_timer.reference_time();

		let elapsed = final_time - initial_time;
		assert_eq!(
			Duration::from_millis(1),
			elapsed,
			"Expected the elapsed time to be 1ms, but was: {elapsed:?}",
		);
	}

	#[test]
	fn should_provide_the_passage_of_reference_time_in_milliseconds() {
		let (clock, clock_mock) = Clock::mock();
		let reference_timer = ReferenceTimer::default().with_clock(clock);

		let initial_milliseconds = reference_timer.reference_time_milliseconds();
		clock_mock.increment(Duration::from_millis(1));
		let final_milliseconds = reference_timer.reference_time_milliseconds();

		let elapsed = final_milliseconds - initial_milliseconds;
		assert_eq!(
			uint!(1),
			elapsed,
			"Expected the elapsed time to be between 1ms, but was: {elapsed}ms",
		);
	}

	#[test]
	fn reference_time_should_resemble_unix_timestamp() {
		let reference_timer = ReferenceTimer::default();
		let now = Utc::now();

		let reference_time = reference_timer.reference_time_milliseconds();

		let unix_timestamp = now.timestamp_millis();
		let reference_timestamp = i64::from(reference_time);

		let diff = (unix_timestamp - reference_timestamp).abs();
		assert!(
			diff < 100,
			"The reference time wasn't close enough to a UNIX timestamp. Diff: {diff} ms"
		);
	}
}
