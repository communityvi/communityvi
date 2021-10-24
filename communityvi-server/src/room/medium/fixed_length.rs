use crate::room::medium::playback_state::PlaybackState;
use chrono::Duration;

/// A medium with a fixed length. e.g. Video file or online video.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedLengthMedium {
	pub length: Duration,
	pub name: String,
	pub playback: PlaybackState,
}

impl FixedLengthMedium {
	pub fn new(name: String, length: Duration) -> Self {
		Self {
			length,
			name,
			playback: PlaybackState::default(),
		}
	}

	pub(super) fn play(&mut self, start_time: Duration, reference_now: Duration) {
		let medium_has_ended = (start_time + self.length) < reference_now;

		self.playback = if medium_has_ended {
			PlaybackState::Paused {
				at_position: self.length,
			}
		} else {
			PlaybackState::Playing { start_time }
		};
	}

	pub(super) fn pause(&mut self, at_position: Duration) {
		let new_position = at_position
			.max(Duration::seconds(0)) // Don't pause before 0
			.min(self.length); // Don't pause after the end

		self.playback = PlaybackState::Paused {
			at_position: new_position,
		};
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn test_medium() -> FixedLengthMedium {
		FixedLengthMedium::new("The Universe".to_string(), Duration::seconds(42))
	}

	#[test]
	fn should_initially_be_paused_at_the_first_position() {
		let medium = test_medium();

		let playback_state = medium.playback;

		assert_eq!(
			playback_state,
			PlaybackState::Paused {
				at_position: Duration::seconds(0)
			},
		);
	}

	#[test]
	fn should_start_playing() {
		let mut medium = test_medium();

		let now = 1337;
		medium.play(Duration::seconds(now), Duration::seconds(now));

		assert_eq!(
			medium.playback,
			PlaybackState::Playing {
				start_time: Duration::seconds(now)
			},
		);
	}

	#[test]
	fn should_not_keep_playing_past_the_end() {
		let mut medium = test_medium();

		let now = 1000;
		medium.play(Duration::seconds(now - 1) - medium.length, Duration::seconds(now));

		assert_eq!(
			medium.playback,
			PlaybackState::Paused {
				at_position: medium.length
			}
		);
	}

	#[test]
	fn should_skip_while_playing() {
		let mut medium = test_medium();
		let now = 1000;
		medium.play(Duration::seconds(now - 1), Duration::seconds(now));

		medium.play(Duration::seconds(now - 10), Duration::seconds(now));

		assert_eq!(
			medium.playback,
			PlaybackState::Playing {
				start_time: Duration::seconds(now - 10)
			}
		);
	}

	#[test]
	fn should_skip_while_paused() {
		let mut medium = test_medium();

		medium.pause(Duration::seconds(13));

		assert_eq!(
			medium.playback,
			PlaybackState::Paused {
				at_position: Duration::seconds(13)
			}
		);
	}

	#[test]
	fn should_pause() {
		let mut medium = test_medium();
		let now = 1000;
		medium.play(Duration::seconds(now - 1), Duration::seconds(now));

		medium.pause(Duration::seconds(1));

		assert_eq!(
			medium.playback,
			PlaybackState::Paused {
				at_position: Duration::seconds(1)
			}
		);
	}

	#[test]
	fn should_not_pause_before_start() {
		let mut medium = test_medium();

		medium.pause(Duration::seconds(-1));

		assert_eq!(
			medium.playback,
			PlaybackState::Paused {
				at_position: Duration::seconds(0)
			}
		);
	}

	#[test]
	fn should_not_pause_after_end() {
		let mut medium = test_medium();

		medium.pause(medium.length + Duration::seconds(1));

		assert_eq!(
			medium.playback,
			PlaybackState::Paused {
				at_position: medium.length
			}
		);
	}
}
