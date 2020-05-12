use crate::room::state::medium::{Medium, VersionedMedium};
use chrono::Duration;
use std::time::Instant;

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
	pub fn current_reference_time(&self) -> std::time::Duration {
		self.start_of_reference_time.elapsed()
	}

	/// Insert a medium based on `previous_version`. If `previous_version` is too low, nothing happens
	/// and `None` is returned. This is similar to compare and swap.
	pub fn insert_medium(&self, medium: Medium, previous_version: u64) -> Option<VersionedMedium> {
		let mut versioned_medium = self.medium.lock();
		if previous_version != versioned_medium.version {
			return None;
		}

		versioned_medium.update(medium);

		Some(versioned_medium.clone())
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn play_medium(&self, start_time: Duration, previous_version: u64) -> Option<VersionedMedium> {
		let reference_now = Duration::from_std(self.current_reference_time())
			.expect("This won't happen unless you run the server for more than 9_223_372_036_854_775_807 seconds :)");
		self.medium.lock().play(start_time, reference_now, previous_version)
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn pause_medium(&self, at_position: Duration, previous_version: u64) -> Option<VersionedMedium> {
		self.medium.lock().pause(at_position, previous_version)
	}

	pub fn medium(&self) -> VersionedMedium {
		self.medium.lock().clone()
	}

	pub fn eject_medium(&self) {
		self.medium.lock().update(Medium::Empty);
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
