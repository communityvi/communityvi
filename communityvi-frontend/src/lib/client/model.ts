import type {
	ChatBroadcast,
	ClientJoinedBroadcast,
	ClientLeftBroadcast,
	FixedLengthMediumBroadcast,
	MediumStateChangedBroadcast,
	VersionedMediumBroadcast,
} from '$lib/client/broadcast';
import {MediumType} from '$lib/client/request';
import type {
	ClientResponse,
	FixedLengthVersionedMediumResponse,
	PausedPlaybackStateResponse,
	PlaybackStateResponse,
	PlayingPlaybackStateResponse,
	VersionedMediumResponse,
} from '$lib/client/response';
import {PlaybackStateType} from '$lib/client/response';
import {LeftReason} from '$lib/client/broadcast';

export class PeerJoinedMessage implements PeerLifecycleMessage {
	readonly peer: Peer;

	static fromClientJoinedBroadcast(broadcast: ClientJoinedBroadcast): PeerJoinedMessage {
		const peer = Peer.fromClientBroadcast(broadcast);
		return new PeerJoinedMessage(peer);
	}

	constructor(peer: Peer) {
		this.peer = peer;
	}
}

export class PeerLeftMessage implements PeerLifecycleMessage {
	readonly peer: Peer;
	readonly reason: LeaveReason;

	static fromClientLeftBroadcast(broadcast: ClientLeftBroadcast): PeerLeftMessage {
		let reason: LeaveReason;
		switch (broadcast.reason) {
			case LeftReason.Closed:
				reason = LeaveReason.Closed;
				break;
			case LeftReason.Timeout:
				reason = LeaveReason.Timeout;
				break;
			default:
				throw new Error(`Invalid LeftReason reason: '${broadcast.reason}'`);
		}

		const peer = Peer.fromClientBroadcast(broadcast);
		return new PeerLeftMessage(peer, reason);
	}

	constructor(peer: Peer, reason: LeaveReason) {
		this.peer = peer;
		this.reason = reason;
	}
}

export enum LeaveReason {
	Closed,
	Timeout,
}

export interface PeerLifecycleMessage {
	readonly peer: Peer;
}

export class ChatMessage {
	readonly message: string;
	readonly sender: Peer;

	static fromChatBroadcast(broadcast: ChatBroadcast): ChatMessage {
		const sender = Peer.fromChatBroadcast(broadcast);
		return new ChatMessage(broadcast.message, sender);
	}

	constructor(message: string, sender: Peer) {
		this.message = message;
		this.sender = sender;
	}
}

export class MediumState {
	readonly version: number;

	// FIXME: Ideally, the server should keep track who did it last so that this information is always available!
	readonly changedBy?: Peer;
	readonly medium?: Medium;

	static fromVersionedMediumResponse(response: VersionedMediumResponse): MediumState {
		const medium = Medium.fromVersionedMediumResponse(response);
		return new MediumState(response.version, undefined, medium);
	}

	static fromMediumStateChangedBroadcast(broadcast: MediumStateChangedBroadcast): MediumState {
		const medium = Medium.fromVersionedMediumBroadcast(broadcast.medium);
		const peer = Peer.fromMediumStateChangedBroadcast(broadcast);
		return new MediumState(broadcast.medium.version, peer, medium);
	}

	constructor(version: number, changedBy?: Peer, medium?: Medium) {
		this.version = version;
		this.changedBy = changedBy;
		this.medium = medium;
	}
}

export class Peer {
	readonly id: number;
	readonly name: string;

	static fromClientResponse(response: ClientResponse): Peer {
		return new Peer(response.id, response.name);
	}

	static fromClientBroadcast(broadcast: ClientLeftBroadcast | ClientJoinedBroadcast): Peer {
		return new Peer(broadcast.id, broadcast.name);
	}

	static fromChatBroadcast(broadcast: ChatBroadcast): Peer {
		return new Peer(broadcast.sender_id, broadcast.sender_name);
	}

	static fromMediumStateChangedBroadcast(broadcast: MediumStateChangedBroadcast): Peer {
		return new Peer(broadcast.changed_by_id, broadcast.changed_by_name);
	}

	constructor(id: number, name: string) {
		this.id = id;
		this.name = name;
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
