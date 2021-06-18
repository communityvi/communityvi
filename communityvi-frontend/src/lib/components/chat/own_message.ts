export class OwnMessage {
	readonly message: string;
	readonly senderId: number;
	readonly senderName: string;

	acknowledged = false;

	constructor(message: string, senderId: number, senderName: string) {
		this.message = message;
		this.senderId = senderId;
		this.senderName = senderName;
	}
}
