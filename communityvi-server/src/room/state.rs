use crate::room::state::medium::Medium;
use parking_lot::MutexGuard;
use std::time::{Duration, Instant};

pub mod medium;

#[derive(Debug)]
pub struct State {
	start_of_reference_time: Instant,
	medium: parking_lot::Mutex<Medium>,
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

	pub fn insert_medium(&self, medium: Medium) {
		*self.medium.lock() = medium;
	}

	pub fn medium(&self) -> MutexGuard<Medium> {
		self.medium.lock()
	}

	pub fn eject_medium(&self) {
		self.insert_medium(Medium::Empty)
	}
}
