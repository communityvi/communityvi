use js_int::UInt;
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct ReferenceTimer(Instant);

impl ReferenceTimer {
	pub fn reference_time(&self) -> std::time::Duration {
		self.0.elapsed()
	}

	pub fn reference_time_milliseconds(&self) -> UInt {
		#[allow(clippy::cast_possible_truncation)]
		let milliseconds = self.reference_time().as_millis() as u64;
		UInt::try_from(milliseconds)
			.expect("More milliseconds than can be represented by IEEE 754 doubles. This shouldn't happen unless the server was running for more than about 285000 years.")
	}
}

impl Default for ReferenceTimer {
	fn default() -> Self {
		Self(Instant::now())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::thread::sleep;
	use std::time::Duration;

	#[test]
	fn reference_timer_should_time_the_reference_time() {
		let reference_timer = ReferenceTimer::default();

		let initial_time = reference_timer.reference_time();
		sleep(Duration::from_millis(1));
		let final_time = reference_timer.reference_time();

		let elapsed = final_time - initial_time;
		assert!(
			(1..10).contains(&elapsed.as_millis()),
			"Expected the elapsed time to be between 1ms and 10ms, but was: {elapsed:?}",
		);
	}

	#[test]
	fn reference_timer_should_time_reference_time_milliseconds() {
		let reference_timer = ReferenceTimer::default();

		let initial_milliseconds = reference_timer.reference_time_milliseconds();
		sleep(Duration::from_millis(1));
		let final_milliseconds = reference_timer.reference_time_milliseconds();

		let elapsed = final_milliseconds - initial_milliseconds;
		assert!(
			(1..10).contains(&u64::from(elapsed)),
			"Expected the elapsed time to be between 1ms and 10ms, but was: {elapsed}ms",
		);
	}
}
