use crate::room::state::medium::SomeMedium;
use std::time::{Duration, Instant};

mod medium;

#[derive(Debug)]
pub struct State {
	start_of_reference_time: Instant,
	medium: Option<SomeMedium>,
}

impl Default for State {
	fn default() -> Self {
		Self {
			start_of_reference_time: Instant::now(),
			medium: None,
		}
	}
}

impl State {
	pub fn current_reference_time(&self) -> Duration {
		self.start_of_reference_time.elapsed()
	}

	pub fn insert_medium(&mut self, some_medium: SomeMedium) {
		self.medium = Some(some_medium);
	}
}
