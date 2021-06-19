import type {PlaybackStateResponse, ServerResponse} from '$lib/client/response';
import type {MediumType} from '$lib/client/request';

export interface MediumStateChangedBroadcast extends BroadcastMessage {
	readonly changed_by_name: string;
	readonly changed_by_id: number;
	readonly medium: VersionedMediumBroadcast;
}

export interface VersionedMediumBroadcast {
	readonly version: number;
	readonly medium: MediumBroadcast;
}

export interface FixedLengthMediumBroadcast extends MediumBroadcast {
	readonly name: string;
	readonly length_in_milliseconds: number;
	readonly playback_skipped: boolean;
	readonly playback_state: PlaybackStateResponse;
}

export interface MediumBroadcast {
	readonly type: MediumType;
}

export interface ChatBroadcast extends BroadcastMessage {
	readonly sender_id: number;
	readonly sender_name: string;
	readonly message: string;
	readonly counter: number;
}

export interface BroadcastResponse extends ServerResponse {
	readonly message: BroadcastMessage;
}

export interface BroadcastMessage {
	readonly type: BroadcastType;
}

export enum BroadcastType {
	ClientJoined = 'client_joined',
	ClientLeft = 'client_left',
	Chat = 'chat',
	MediumStateChanged = 'medium_state_changed',
}
