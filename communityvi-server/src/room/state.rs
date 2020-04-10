use std::time::{Duration, Instant};

pub struct State {
	start_of_reference_time: Instant,
}

impl Default for State {
	fn default() -> Self {
		Self {
			start_of_reference_time: Instant::now(),
		}
	}
}

impl State {
	pub fn current_reference_time(&self) -> Duration {
		self.start_of_reference_time.elapsed()
	}
}
