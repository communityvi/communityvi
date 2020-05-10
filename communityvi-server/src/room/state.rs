use crate::room::state::medium::{Medium, VersionedMedium};
use parking_lot::MutexGuard;
use std::time::{Duration, Instant};

pub mod medium;

#[derive(Debug)]
pub struct State {
	start_of_reference_time: Instant,
	medium: parking_lot::Mutex<VersionedMedium>,
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

	/// Insert a medium based on `previous_version`. If `previous_version` is too low, nothing happens
	/// and `None` is returned. This is similar to compare and swap.
	pub fn insert_medium(&self, medium: Medium, previous_version: u64) -> Option<VersionedMedium> {
		let mut versioned_medium = self.medium();
		if previous_version != versioned_medium.version {
			return None;
		}

		versioned_medium.update(medium);

		Some(versioned_medium.clone())
	}

	pub fn medium(&self) -> MutexGuard<VersionedMedium> {
		self.medium.lock()
	}

	pub fn eject_medium(&self) {
		self.medium().update(Medium::Empty);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn should_not_insert_medium_with_smaller_previous_version() {
		let state = State::default();
		state.insert_medium(Medium::Empty, 0).expect("Failed to insert medium"); // increase the version
		assert_eq!(state.medium().version, 1);

		assert!(
			state.insert_medium(Medium::Empty, 0).is_none(),
			"Must not be able to insert"
		);
		assert_eq!(state.medium().version, 1);
	}

	#[test]
	fn should_not_insert_medium_with_larger_previous_version() {
		let state = State::default();
		assert!(
			state.insert_medium(Medium::Empty, 1).is_none(),
			"Must not be able to insert"
		);
		assert_eq!(state.medium().version, 0);
	}
}
