use js_int::{Int, UInt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::room::client::Client;
use crate::room::medium::playback_state::PlaybackState;
use crate::room::medium::{Medium, VersionedMedium};
use crate::room::session_id::SessionId;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ClientResponse {
	pub id: SessionId,
	pub name: String,
}

impl From<Client> for ClientResponse {
	fn from(client: Client) -> Self {
		Self {
			id: client.id(),
			name: client.name().to_string(),
		}
	}
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VersionedMediumResponse {
	pub version: UInt,
	#[serde(flatten)]
	pub medium: MediumResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum MediumResponse {
	FixedLength {
		name: String,
		length_in_milliseconds: u64,
		playback_state: PlaybackStateResponse,
	},
	Empty,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum PlaybackStateResponse {
	Playing {
		#[schemars(with = "i64")]
		start_time_in_milliseconds: Int,
	},
	Paused {
		#[schemars(with = "u64")]
		position_in_milliseconds: UInt,
	},
}

impl From<PlaybackState> for PlaybackStateResponse {
	fn from(playback_state: PlaybackState) -> Self {
		match playback_state {
			PlaybackState::Playing { start_time } => Self::Playing {
				start_time_in_milliseconds: Int::try_from(start_time.num_milliseconds()).unwrap(),
			},
			PlaybackState::Paused { at_position } => Self::Paused {
				position_in_milliseconds: UInt::try_from(at_position.num_milliseconds()).unwrap(),
			},
		}
	}
}

impl From<VersionedMedium> for VersionedMediumResponse {
	fn from(versioned_medium: VersionedMedium) -> Self {
		Self {
			medium: versioned_medium.medium.into(),
			version: versioned_medium.version,
		}
	}
}

impl From<Medium> for MediumResponse {
	fn from(medium: Medium) -> Self {
		match medium {
			Medium::FixedLength(fixed_length) => MediumResponse::FixedLength {
				name: fixed_length.name,
				length_in_milliseconds: u64::try_from(fixed_length.length.num_milliseconds()).unwrap(),
				playback_state: fixed_length.playback.into(),
			},
			Medium::Empty => MediumResponse::Empty,
		}
	}
}
