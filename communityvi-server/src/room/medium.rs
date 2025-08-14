use crate::room::medium::fixed_length::FixedLengthMedium;
use chrono::Duration;
use js_int::{UInt, uint};

pub mod fixed_length;
pub mod model;
pub mod playback_state;
pub mod repository;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VersionedMedium {
	pub version: UInt,
	pub medium: Medium,
}

impl VersionedMedium {
	pub fn update(&mut self, medium: Medium) {
		self.version += uint!(1);
		self.medium = medium;
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Medium {
	#[default]
	Empty,
	FixedLength(FixedLengthMedium),
}

impl VersionedMedium {
	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub(super) fn play(
		&mut self,
		start_time: Duration,
		reference_now: Duration,
		previous_version: UInt,
	) -> Option<VersionedMedium> {
		if self.version != previous_version {
			return None;
		}
		match &mut self.medium {
			Medium::Empty => {}
			Medium::FixedLength(medium) => {
				medium.play(start_time, reference_now);
			}
		}
		self.version += uint!(1);
		Some(self.clone())
	}

	#[must_use = "returns a `VersionedMedium` with new version that must be propagated"]
	pub(super) fn pause(&mut self, at_position: Duration, previous_version: UInt) -> Option<VersionedMedium> {
		if self.version != previous_version {
			return None;
		}

		match &mut self.medium {
			Medium::Empty => {}
			Medium::FixedLength(medium) => {
				medium.pause(at_position);
			}
		}
		self.version += uint!(1);
		Some(self.clone())
	}
}

impl From<FixedLengthMedium> for Medium {
	fn from(fixed_length_medium: FixedLengthMedium) -> Self {
		Medium::FixedLength(fixed_length_medium)
	}
}

#[cfg(test)]
mod test {
	use crate::room::medium::VersionedMedium;
	use chrono::Duration;
	use js_int::uint;

	#[test]
	fn play_should_increase_the_version() {
		let mut versioned_medium = VersionedMedium::default();
		assert_eq!(versioned_medium.version, uint!(0));
		let returned_versioned_medium = versioned_medium
			.play(Duration::milliseconds(0), Duration::milliseconds(0), uint!(0))
			.expect("Failed to play");
		assert_eq!(versioned_medium.version, uint!(1));
		assert_eq!(versioned_medium, returned_versioned_medium);
	}

	#[test]
	fn play_should_not_work_with_smaller_version() {
		let mut versioned_medium = VersionedMedium {
			version: uint!(1),
			medium: Default::default(),
		};
		assert!(
			versioned_medium
				.play(Duration::milliseconds(0), Duration::milliseconds(0), uint!(0))
				.is_none(),
			"Must not be able to play"
		);
		assert_eq!(versioned_medium.version, uint!(1));
	}

	#[test]
	fn play_should_not_work_with_larger_version() {
		let mut versioned_medium = VersionedMedium::default();
		assert!(
			versioned_medium
				.play(Duration::milliseconds(0), Duration::milliseconds(0), uint!(1))
				.is_none(),
			"Must not be able to play"
		);
		assert_eq!(versioned_medium.version, uint!(0));
	}

	#[test]
	fn pause_should_increase_the_version() {
		let mut versioned_medium = VersionedMedium::default();
		assert_eq!(versioned_medium.version, uint!(0));
		let returned_versioned_medium = versioned_medium
			.pause(Duration::milliseconds(0), uint!(0))
			.expect("Failed to pause");
		assert_eq!(versioned_medium.version, uint!(1));
		assert_eq!(versioned_medium, returned_versioned_medium);
	}

	#[test]
	fn pause_should_not_work_with_smaller_version() {
		let mut versioned_medium = VersionedMedium {
			version: uint!(1),
			medium: Default::default(),
		};
		assert!(
			versioned_medium.pause(Duration::milliseconds(0), uint!(0)).is_none(),
			"Must not be able to pause"
		);
		assert_eq!(versioned_medium.version, uint!(1));
	}

	#[test]
	fn pause_should_not_work_with_larger_version() {
		let mut versioned_medium = VersionedMedium::default();
		assert!(
			versioned_medium.pause(Duration::milliseconds(0), uint!(1)).is_none(),
			"Must not be able to pause"
		);
		assert_eq!(versioned_medium.version, uint!(0));
	}
}
