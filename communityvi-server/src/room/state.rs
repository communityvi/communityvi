use crate::room::state::medium::SomeMedium;
use parking_lot::MutexGuard;
use std::time::{Duration, Instant};

pub mod medium;

#[derive(Debug)]
pub struct State {
	start_of_reference_time: Instant,
	medium: parking_lot::Mutex<Option<SomeMedium>>,
}

impl Default for State {
	fn default() -> Self {
		Self {
			start_of_reference_time: Instant::now(),
			medium: Default::default(),
		}
	}
}

impl State {
	pub fn current_reference_time(&self) -> Duration {
		self.start_of_reference_time.elapsed()
	}

	pub fn insert_medium(&self, some_medium: SomeMedium) {
		*self.medium.lock() = Some(some_medium);
	}

	pub fn medium(&self) -> MutexGuard<Option<SomeMedium>> {
		self.medium.lock()
	}
}
