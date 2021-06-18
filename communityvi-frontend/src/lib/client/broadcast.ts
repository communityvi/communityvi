import type {ServerResponse} from '$lib/client/response';

export interface BroadcastResponse extends ServerResponse {
	readonly message: BroadcastMessage;
}

export interface BroadcastMessage {
	readonly type: BroadcastType;
}

export interface ChatBroadcast extends BroadcastMessage {
	readonly sender_id: number;
	readonly sender_name: string;
	readonly message: string;
	readonly counter: number;
}

export enum BroadcastType {
	ClientJoined = 'client_joined',
	ClientLeft = 'client_left',
	Chat = 'chat',
	MediumStateChanged = 'medium_state_changed',
}
