import type {HelloMessage, ServerResponse} from '$lib/client/response';
import {ChatBroadcast, BroadcastMessage, BroadcastType} from '$lib/client/broadcast';
import {ChatRequest, RegisterRequest} from '$lib/client/request';
import type {Transport} from '$lib/client/transport';
import type {CloseReason, Connection} from '$lib/client/connection';
import {ChatMessage} from '$lib/client/model';

export class Client {
	readonly transport: Transport;

	constructor(transport: Transport) {
		this.transport = transport;
	}

	async register(name: string, disconnectCallback: DisconnectCallback): Promise<RegisteredClient> {
		const connection = await this.transport.connect();

		const response = (await connection.performRequest(new RegisterRequest(name))) as HelloMessage;

		return new RegisteredClient(response.id, name, connection, disconnectCallback);
	}
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;

	private readonly connection: Connection;
	private readonly disconnectCallback: DisconnectCallback;
	private readonly chatMessageCallbacks = new Array<ChatMessageCallback>();

	constructor(id: number, name: string, connection: Connection, disconnectCallback: DisconnectCallback) {
		this.id = id;
		this.name = name;
		this.connection = connection;
		this.disconnectCallback = disconnectCallback;

		this.connection.setDelegate({
			connectionDidReceiveBroadcast: response => this.connectionDidReceiveBroadcast(response),
			connectionDidReceiveUnassignableResponse: response => this.connectionDidReceiveUnassignableResponse(response),
			connectionDidClose: reason => this.connectionDidClose(reason),
			connectionDidEncounterError: error => this.connectionDidEncounterError(error),
		});
	}

	logout(): void {
		this.connection.disconnect();
	}

	async sendChatMessage(message: string): Promise<void> {
		await this.connection.performRequest(new ChatRequest(message));
	}

	subscribeToChatMessages(callback: ChatMessageCallback): void {
		this.chatMessageCallbacks.push(callback);
	}

	private connectionDidReceiveBroadcast(broadcast: BroadcastMessage): void {
		console.info('Received broadcast:', broadcast);

		switch (broadcast.type) {
			case BroadcastType.Chat: {
				const chatBroadcast = broadcast as ChatBroadcast;
				if (chatBroadcast.sender_id === this.id) {
					// we already know what message we've sent ourselves
					return;
				}

				const chatMessage = ChatMessage.fromChatBroadcast(chatBroadcast);
				for (const chatMessageCallback of this.chatMessageCallbacks) {
					chatMessageCallback(chatMessage);
				}

				break;
			}
		}
	}

	private connectionDidReceiveUnassignableResponse(response: ServerResponse): void {
		console.warn('Received unassignable response:', response);
	}

	private connectionDidClose(reason: CloseReason): void {
		this.disconnectCallback(reason);
	}

	private connectionDidEncounterError(error: Event | ErrorEvent): void {
		console.error('Received error:', error);
	}
}

export type DisconnectCallback = (reason: CloseReason) => void;
export type ChatMessageCallback = (message: ChatMessage) => void;
