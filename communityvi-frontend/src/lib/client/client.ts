import type {HelloMessage, ServerResponse} from '$lib/client/response';
import {BroadcastMessage, BroadcastType, ChatBroadcast, MediumStateChangedBroadcast} from '$lib/client/broadcast';
import {ChatRequest, EmptyMedium, FixedLengthMedium, InsertMediumRequest, RegisterRequest} from '$lib/client/request';
import type {Transport} from '$lib/client/transport';
import type {CloseReason, Connection} from '$lib/client/connection';
import {ChatMessage, MediumState} from '$lib/client/model';

export class Client {
	readonly transport: Transport;

	constructor(transport: Transport) {
		this.transport = transport;
	}

	async register(name: string, disconnectCallback: DisconnectCallback): Promise<RegisteredClient> {
		const connection = await this.transport.connect();

		const response = (await connection.performRequest(new RegisterRequest(name))) as HelloMessage;
		const mediumState = MediumState.fromVersionedMediumResponse(response.current_medium);

		return new RegisteredClient(response.id, name, mediumState, connection, disconnectCallback);
	}
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;
	private currentMediumState: MediumState;

	private readonly connection: Connection;
	private readonly disconnectCallback: DisconnectCallback;

	private readonly chatMessageCallbacks = new Array<ChatMessageCallback>();
	private readonly mediumStateChangedCallbacks = new Array<MediumStateChangedCallback>();

	constructor(
		id: number,
		name: string,
		currentMediumState: MediumState,
		connection: Connection,
		disconnectCallback: DisconnectCallback,
	) {
		this.id = id;
		this.name = name;
		this.currentMediumState = currentMediumState;

		this.connection = connection;
		this.disconnectCallback = disconnectCallback;

		this.connection.setDelegate({
			connectionDidReceiveBroadcast: response => this.connectionDidReceiveBroadcast(response),
			connectionDidReceiveUnassignableResponse: response => this.connectionDidReceiveUnassignableResponse(response),
			connectionDidClose: reason => this.connectionDidClose(reason),
			connectionDidEncounterError: error => this.connectionDidEncounterError(error),
		});
	}

	async sendChatMessage(message: string): Promise<void> {
		await this.connection.performRequest(new ChatRequest(message));
	}

	subscribeToChatMessages(callback: ChatMessageCallback): Unsubscriber {
		return RegisteredClient.subscribe(callback, this.chatMessageCallbacks);
	}

	getCurrentMediumState(): MediumState {
		return this.currentMediumState;
	}

	async insertMedium(previousVersion: number, medium: FixedLengthMedium): Promise<number> {
		await this.connection.performRequest(new InsertMediumRequest(previousVersion, medium));
		// FIXME: We anticipate the upcoming improved(tm) REST-API to handle this.
		return previousVersion + 1;
	}

	async ejectMedium(previousVersion: number): Promise<number> {
		await this.connection.performRequest(new InsertMediumRequest(previousVersion, new EmptyMedium()));
		// FIXME: We anticipate the upcoming improved(tm) REST-API to handle this.
		return previousVersion + 1;
	}

	subscribeToMediumStateChanges(callback: MediumStateChangedCallback): Unsubscriber {
		return RegisteredClient.subscribe(callback, this.mediumStateChangedCallbacks);
	}

	private static subscribe<Callback>(callback: Callback, callbackList: Array<Callback>): Unsubscriber {
		callbackList.push(callback);

		return () => {
			const index = callbackList.indexOf(callback);
			if (index === -1) {
				return;
			}

			callbackList.slice(index, 1);
		};
	}

	logout(): void {
		this.connection.disconnect();
	}

	private connectionDidReceiveBroadcast(broadcast: BroadcastMessage): void {
		console.info('Received broadcast:', broadcast);

		switch (broadcast.type) {
			case BroadcastType.Chat: {
				const chatBroadcast = broadcast as ChatBroadcast;
				if (this.id === chatBroadcast.sender_id) {
					// we already know what message we've sent ourselves
					return;
				}

				const chatMessage = ChatMessage.fromChatBroadcast(chatBroadcast);
				for (const chatMessageCallback of this.chatMessageCallbacks) {
					chatMessageCallback(chatMessage);
				}

				break;
			}
			case BroadcastType.MediumStateChanged: {
				const mediumStateChangedBroadcast = broadcast as MediumStateChangedBroadcast;
				const mediumState = MediumState.fromMediumStateChangedBroadcast(mediumStateChangedBroadcast);
				this.currentMediumState = mediumState;

				if (this.id === mediumState.changedById) {
					// we already know about the changes we've made and implicitly updated the state of the client.
					return;
				}

				for (const mediumStateChangedCallback of this.mediumStateChangedCallbacks) {
					mediumStateChangedCallback(mediumState);
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
export type MediumStateChangedCallback = (mediumState: MediumState) => void;

export type Unsubscriber = () => void;
