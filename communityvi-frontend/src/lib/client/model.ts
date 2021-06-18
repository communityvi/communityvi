import type {ChatBroadcast} from '$lib/client/broadcast';

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
