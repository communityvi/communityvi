import type {PlaybackStateResponse, ServerResponse} from '$lib/client/response';
import type {MediumType} from '$lib/client/request';
import {Peer} from "$lib/client/model";

export interface ClientLeftBroadcast extends BroadcastMessage {
	readonly id: number;
	readonly name: string;
	readonly reason: LeftReason;
}

export enum LeftReason {
	Closed = 'closed',
	Timeout = 'timeout',
}

export interface ClientJoinedBroadcast extends BroadcastMessage {
	readonly id: number;
	readonly name: string;
	readonly participants: Participant[];
}

export interface Participant {
	readonly id: number;
	readonly name: string;
}

export interface MediumStateChangedBroadcast extends BroadcastMessage {
	readonly changed_by_name: string;
	readonly changed_by_id: number;
	readonly medium: VersionedMediumBroadcast;
}

export interface VersionedMediumBroadcast {
	readonly type: MediumType;
	readonly version: number;
}

export interface FixedLengthMediumBroadcast extends VersionedMediumBroadcast {
	readonly name: string;
	readonly length_in_milliseconds: number;
	readonly playback_skipped: boolean;
	readonly playback_state: PlaybackStateResponse;
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
