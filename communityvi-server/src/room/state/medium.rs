use crate::room::state::medium::fixed_length::FixedLengthMedium;
use chrono::Duration;

pub mod fixed_length;
pub mod playback_state;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VersionedMedium {
	pub version: u64,
	pub medium: Medium,
}

impl VersionedMedium {
	pub fn update(&mut self, medium: Medium) {
		self.version += 1;
		self.medium = medium;
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Medium {
	Empty,
	FixedLength(FixedLengthMedium),
}

impl Default for Medium {
	fn default() -> Self {
		Medium::Empty
	}
}

impl VersionedMedium {
	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn play(&mut self, start_time: Duration, reference_now: Duration) -> VersionedMedium {
		match &mut self.medium {
			Medium::Empty => {}
			Medium::FixedLength(medium) => {
				medium.play(start_time, reference_now);
			}
		}
		self.version += 1;
		self.clone()
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub fn pause(&mut self, at_position: Duration) -> VersionedMedium {
		match &mut self.medium {
			Medium::Empty => {}
			Medium::FixedLength(medium) => {
				medium.pause(at_position);
			}
		}
		self.version += 1;
		self.clone()
	}
}

impl From<FixedLengthMedium> for Medium {
	fn from(fixed_length_medium: FixedLengthMedium) -> Self {
		Medium::FixedLength(fixed_length_medium)
	}
}

#[cfg(test)]
mod test {
	use crate::room::state::medium::VersionedMedium;
	use chrono::Duration;

	#[test]
	fn play_should_increase_the_version() {
		let mut versioned_medium = VersionedMedium::default();
		assert_eq!(versioned_medium.version, 0);
		let returned_versioned_medium = versioned_medium.play(Duration::milliseconds(0), Duration::milliseconds(0));
		assert_eq!(versioned_medium.version, 1);
		assert_eq!(versioned_medium, returned_versioned_medium);
	}

	#[test]
	fn pause_should_increase_the_version() {
		let mut versioned_medium = VersionedMedium::default();
		assert_eq!(versioned_medium.version, 0);
		let returned_versioned_medium = versioned_medium.pause(Duration::milliseconds(0));
		assert_eq!(versioned_medium.version, 1);
		assert_eq!(versioned_medium, returned_versioned_medium);
	}
}
