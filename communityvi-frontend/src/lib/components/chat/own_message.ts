import type {Peer} from '$lib/client/model';

export class OwnMessage {
	readonly message: string;
	readonly sender: Peer;

	acknowledged = false;

	constructor(message: string, sender: Peer) {
		this.message = message;
		this.sender = sender;
	}
}
