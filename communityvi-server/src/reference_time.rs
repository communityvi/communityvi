use js_int::UInt;
use quanta::Clock;

#[derive(Clone)]
pub struct ReferenceTimer {
	start: quanta::Instant,
	clock: Clock,
}

impl ReferenceTimer {
	pub fn reference_time(&self) -> std::time::Duration {
		self.clock.now().duration_since(self.start)
	}

	pub fn reference_time_milliseconds(&self) -> UInt {
		#[allow(clippy::cast_possible_truncation)]
		let milliseconds = self.reference_time().as_millis() as u64;
		UInt::try_from(milliseconds)
			.expect("More milliseconds than can be represented by IEEE 754 doubles. This shouldn't happen unless the server was running for more than about 285000 years.")
	}

	#[cfg(test)]
	fn with_clock(clock: Clock) -> Self {
		Self {
			start: clock.now(),
			clock,
		}
	}
}

impl Default for ReferenceTimer {
	fn default() -> Self {
		let clock = Clock::default();
		Self {
			start: clock.now(),
			clock,
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
		let reference_timer = ReferenceTimer::with_clock(clock);

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
		let reference_timer = ReferenceTimer::with_clock(clock);

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
}
