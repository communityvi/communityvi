import type {
	ChatBroadcast,
	FixedLengthMediumBroadcast,
	MediumStateChangedBroadcast,
	VersionedMediumBroadcast,
} from '$lib/client/broadcast';
import {MediumType} from '$lib/client/request';
import type {
	FixedLengthVersionedMediumResponse,
	PausedPlaybackStateResponse,
	PlaybackStateResponse,
	PlayingPlaybackStateResponse,
	VersionedMediumResponse,
} from '$lib/client/response';
import {PlaybackStateType} from '$lib/client/response';

export class ChatMessage {
	readonly message: string;
	readonly senderName: string;
	readonly senderId: number;

	static fromChatBroadcast(broadcast: ChatBroadcast): ChatMessage {
		return new ChatMessage(broadcast.message, broadcast.sender_name, broadcast.sender_id);
	}

	constructor(message: string, senderName: string, senderId: number) {
		this.message = message;
		this.senderName = senderName;
		this.senderId = senderId;
	}
}

export class MediumState {
	readonly version: number;

	// FIXME: Ideally, the server should keep track who did it last so that this information is always available!
	readonly changedByName?: string;
	readonly changedById?: number;

	readonly medium?: Medium;

	static fromVersionedMediumResponse(response: VersionedMediumResponse): MediumState {
		const medium = Medium.fromVersionedMediumResponse(response);
		return new MediumState(response.version, undefined, undefined, medium);
	}

	static fromMediumStateChangedBroadcast(broadcast: MediumStateChangedBroadcast): MediumState {
		const medium = Medium.fromVersionedMediumBroadcast(broadcast.medium);
		return new MediumState(broadcast.medium.version, broadcast.changed_by_name, broadcast.changed_by_id, medium);
	}

	constructor(version: number, changedByName?: string, changedById?: number, medium?: Medium) {
		this.version = version;
		this.changedByName = changedByName;
		this.changedById = changedById;
		this.medium = medium;
	}
}

export class Medium {
	readonly name: string;
	readonly lengthInMilliseconds: number;
	readonly playbackSkipped: boolean;
	readonly playbackState: PlayingPlaybackState | PausedPlaybackState;

	static fromVersionedMediumResponse(response: VersionedMediumResponse): Medium | undefined {
		switch (response.type) {
			case MediumType.FixedLength: {
				const fixedLength = response as FixedLengthVersionedMediumResponse;
				return new Medium(
					fixedLength.name,
					fixedLength.length_in_milliseconds,
					false,
					PlaybackState.fromPlaybackStateResponse(fixedLength.playback_state),
				);
			}
			case MediumType.Empty:
				return undefined;
			default:
				throw new Error(`Invalid MediumResponse type: '${response.type}'`);
		}
	}

	static fromVersionedMediumBroadcast(broadcast: VersionedMediumBroadcast): Medium | undefined {
		switch (broadcast.type) {
			case MediumType.FixedLength: {
				const fixedLength = broadcast as FixedLengthMediumBroadcast;
				return new Medium(
					fixedLength.name,
					fixedLength.length_in_milliseconds,
					fixedLength.playback_skipped,
					PlaybackState.fromPlaybackStateResponse(fixedLength.playback_state),
				);
			}
			case MediumType.Empty:
				return undefined;
			default:
				throw new Error(`Invalid MediumBroadcast type: '${broadcast.type}'`);
		}
	}

	constructor(
		name: string,
		lengthInMilliseconds: number,
		playbackSkipped: boolean,
		playbackState: PlayingPlaybackState | PausedPlaybackState,
	) {
		this.name = name;
		this.lengthInMilliseconds = lengthInMilliseconds;
		this.playbackSkipped = playbackSkipped;
		this.playbackState = playbackState;
	}
}

abstract class PlaybackState {
	static fromPlaybackStateResponse(response: PlaybackStateResponse): PlayingPlaybackState | PausedPlaybackState {
		switch (response.type) {
			case PlaybackStateType.Playing: {
				const playing = response as PlayingPlaybackStateResponse;
				return new PlayingPlaybackState(playing.start_time_in_milliseconds);
			}
			case PlaybackStateType.Paused: {
				const paused = response as PausedPlaybackStateResponse;
				return new PausedPlaybackState(paused.position_in_milliseconds);
			}
			default:
				throw new Error(`Invalid PlaybackState type: '${response.type}'`);
		}
	}
}

class PlayingPlaybackState {
	readonly startTimeInMilliseconds: number;

	constructor(startTimeInMilliseconds: number) {
		this.startTimeInMilliseconds = startTimeInMilliseconds;
	}
}

class PausedPlaybackState {
	readonly positionInMilliseconds: number;

	constructor(positionInMilliseconds: number) {
		this.positionInMilliseconds = positionInMilliseconds;
	}
}
