import type {HelloMessage, ServerResponse} from '$lib/client/response';
import {
	BroadcastMessage,
	BroadcastType,
	ChatBroadcast,
	ClientJoinedBroadcast,
	ClientLeftBroadcast,
	MediumStateChangedBroadcast,
} from '$lib/client/broadcast';
import {ChatRequest, EmptyMedium, FixedLengthMedium, InsertMediumRequest, RegisterRequest} from '$lib/client/request';
import type {Transport} from '$lib/client/transport';
import type {CloseReason, Connection} from '$lib/client/connection';
import {
	ChatMessage,
	PeerLeftMessage,
	MediumState,
	Peer,
	PeerLifecycleMessage,
	PeerJoinedMessage,
} from '$lib/client/model';

export class Client {
	readonly transport: Transport;

	constructor(transport: Transport) {
		this.transport = transport;
	}

	async register(name: string, disconnectCallback: DisconnectCallback): Promise<RegisteredClient> {
		const connection = await this.transport.connect();

		const response = (await connection.performRequest(new RegisterRequest(name))) as HelloMessage;
		const mediumState = MediumState.fromVersionedMediumResponse(response.current_medium);
		const peers = response.clients.map(Peer.fromClientResponse);

		return new RegisteredClient(response.id, name, mediumState, peers, connection, disconnectCallback);
	}
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;
	private currentMediumState: MediumState;
	readonly peers: Array<Peer>;

	private readonly connection: Connection;
	private readonly disconnectCallback: DisconnectCallback;

	private readonly peerLifecycleCallbacks = new Array<PeerLifecycleCallback>();
	private readonly chatMessageCallbacks = new Array<ChatMessageCallback>();
	private readonly mediumStateChangedCallbacks = new Array<MediumStateChangedCallback>();

	constructor(
		id: number,
		name: string,
		currentMediumState: MediumState,
		peers: Array<Peer>,
		connection: Connection,
		disconnectCallback: DisconnectCallback,
	) {
		this.id = id;
		this.name = name;
		this.currentMediumState = currentMediumState;
		this.peers = peers;

		this.connection = connection;
		this.disconnectCallback = disconnectCallback;

		this.connection.setDelegate({
			connectionDidReceiveBroadcast: response => this.connectionDidReceiveBroadcast(response),
			connectionDidReceiveUnassignableResponse: response => this.connectionDidReceiveUnassignableResponse(response),
			connectionDidClose: reason => this.connectionDidClose(reason),
			connectionDidEncounterError: error => this.connectionDidEncounterError(error),
		});
	}

	asPeer(): Peer {
		return new Peer(this.id, this.name);
	}

	subscribeToPeerChanges(callback: PeerLifecycleCallback): Unsubscriber {
		return RegisteredClient.subscribe(callback, this.peerLifecycleCallbacks);
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

	async insertFixedLengthMedium(name: string, lengthInMilliseconds: number): Promise<void> {
		const medium = new FixedLengthMedium(name, lengthInMilliseconds);
		await this.connection.performRequest(new InsertMediumRequest(this.currentMediumState.version, medium));
	}

	async ejectMedium(): Promise<void> {
		await this.connection.performRequest(new InsertMediumRequest(this.currentMediumState.version, new EmptyMedium()));
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
			case BroadcastType.ClientJoined: {
				const clientJoinedBroadcast = broadcast as ClientJoinedBroadcast;
				if (this.id === clientJoinedBroadcast.id) {
					// we already know that we've joined ourselves
					return;
				}

				this.peers.push(Peer.fromClientBroadcast(clientJoinedBroadcast));

				const peerLifecycleMessage = PeerJoinedMessage.fromClientJoinedBroadcast(clientJoinedBroadcast);
				RegisteredClient.notify(peerLifecycleMessage, this.peerLifecycleCallbacks);

				break;
			}
			case BroadcastType.ClientLeft: {
				const clientLeftBroadcast = broadcast as ClientLeftBroadcast;
				const index = this.peers.findIndex(peer => peer.id === clientLeftBroadcast.id);
				if (index === -1) {
					console.error('Unknown peer left:', clientLeftBroadcast);
					return;
				}

				this.peers.slice(index, 1);

				const peerLifecycleMessage = PeerLeftMessage.fromClientLeftBroadcast(clientLeftBroadcast);
				RegisteredClient.notify(peerLifecycleMessage, this.peerLifecycleCallbacks);

				break;
			}
			case BroadcastType.Chat: {
				const chatBroadcast = broadcast as ChatBroadcast;
				if (this.id === chatBroadcast.sender_id) {
					// we already know what message we've sent ourselves
					return;
				}

				const chatMessage = ChatMessage.fromChatBroadcast(chatBroadcast);
				RegisteredClient.notify(chatMessage, this.chatMessageCallbacks);

				break;
			}
			case BroadcastType.MediumStateChanged: {
				const mediumStateChangedBroadcast = broadcast as MediumStateChangedBroadcast;
				const mediumState = MediumState.fromMediumStateChangedBroadcast(mediumStateChangedBroadcast);
				this.currentMediumState = mediumState;

				if (this.id === mediumState.changedBy?.id) {
					// we already know about the changes we've made and implicitly updated the state of the client.
					return;
				}

				RegisteredClient.notify(mediumState, this.mediumStateChangedCallbacks);

				break;
			}
		}
	}

	private static notify<Message>(message: Message, callbackList: Array<(message: Message) => void>) {
		for (const callback of callbackList) {
			callback(message);
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
export type PeerLifecycleCallback = (peerChange: PeerLifecycleMessage) => void;
export type ChatMessageCallback = (message: ChatMessage) => void;
export type MediumStateChangedCallback = (mediumState: MediumState) => void;

export type Unsubscriber = () => void;
