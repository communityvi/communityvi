import type {HelloMessage, ReferenceTimeMessage, ServerResponse} from '$lib/client/response';
import {
	BroadcastMessage,
	BroadcastType,
	ChatBroadcast,
	ClientJoinedBroadcast,
	ClientLeftBroadcast,
	MediumStateChangedBroadcast,
} from '$lib/client/broadcast';
import {
	ChatRequest,
	EmptyMedium,
	FixedLengthMedium,
	GetReferenceTimeRequest,
	InsertMediumRequest,
	PauseRequest,
	PlayRequest,
	RegisterRequest,
} from '$lib/client/request';
import type {Transport} from '$lib/client/transport';
import type {CloseReason, Connection} from '$lib/client/connection';
import {
	ChatMessage,
	PeerLeftMessage,
	Peer,
	PeerLifecycleMessage,
	PeerJoinedMessage,
	Medium,
	VersionedMedium,
	MediumChangedByPeer,
	MediumTimeAdjusted,
	PlayingPlaybackState, PausedPlaybackState,
} from '$lib/client/model';

export class Client {
	readonly transport: Transport;

	constructor(transport: Transport) {
		this.transport = transport;
	}

	async register(name: string, disconnectCallback: DisconnectCallback): Promise<RegisteredClient> {
		const connection = await this.transport.connect();

		const response = (await connection.performRequest(new RegisterRequest(name))).response as HelloMessage;
		const peers = response.clients.map(Peer.fromClientResponse);

		const initialReferenceTimeOffset = await fetchReferenceTimeAndCalculateOffset(connection);
		const versionedMedium = VersionedMedium.fromVersionedMediumResponseAndReferenceTimeOffset(
			response.current_medium,
			initialReferenceTimeOffset,
		);

		return new RegisteredClient(
			response.id,
			name,
			initialReferenceTimeOffset,
			versionedMedium,
			peers,
			connection,
			disconnectCallback,
		);
	}
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;
	private referenceTimeOffset: number;
	private versionedMedium: VersionedMedium;
	readonly peers: Array<Peer>;

	private readonly connection: Connection;
	private readonly disconnectCallback: DisconnectCallback;
	private readonly referenceTimeUpdateIntervalId: NodeJS.Timeout;

	private readonly peerLifecycleCallbacks = new Array<PeerLifecycleCallback>();
	private readonly chatMessageCallbacks = new Array<ChatMessageCallback>();
	private readonly mediumStateChangedCallbacks = new Array<MediumStateChangedCallback>();

	get currentMedium(): Medium | undefined {
		return this.versionedMedium.medium;
	}

	constructor(
		id: number,
		name: string,
		referenceTimeOffset: number,
		versionedMedium: VersionedMedium,
		peers: Array<Peer>,
		connection: Connection,
		disconnectCallback: DisconnectCallback,
	) {
		this.id = id;
		this.name = name;
		this.referenceTimeOffset = referenceTimeOffset;
		this.versionedMedium = versionedMedium;
		this.peers = peers;

		this.connection = connection;
		this.disconnectCallback = disconnectCallback;

		this.connection.setDelegate({
			connectionDidReceiveBroadcast: response => this.connectionDidReceiveBroadcast(response),
			connectionDidReceiveUnassignableResponse: response => this.connectionDidReceiveUnassignableResponse(response),
			connectionDidClose: reason => this.connectionDidClose(reason),
			connectionDidEncounterError: error => this.connectionDidEncounterError(error),
		});

		// Schedule reference time updates every 15s
		this.referenceTimeUpdateIntervalId = setInterval(() => this.synchronizeReferenceTime(), 15_000);
	}

	private async synchronizeReferenceTime(): Promise<void> {
		// FIXME: This is far from ideal. Think about how to make this more cohesive and less ad-hoc.
		console.log('Reference time offset updated:', this.referenceTimeOffset);
		const newOffset = await fetchReferenceTimeAndCalculateOffset(this.connection);
		if (this.referenceTimeOffset === newOffset) {
			console.info('Reference time did not need updating.');
			return;
		}

		const oldOffset = this.referenceTimeOffset;
		this.referenceTimeOffset = newOffset;

		const medium = this.versionedMedium.medium;
		const playbackState = medium?.playbackState;
		if (medium !== undefined && playbackState instanceof PlayingPlaybackState) {
			const newStartTime = playbackState.localStartTimeInMilliseconds + (newOffset - oldOffset);
			const newPlayingPlaybackState = new PlayingPlaybackState(newStartTime);
			const newMedium = new Medium(
				medium.name,
				medium.lengthInMilliseconds,
				medium.playbackSkipped,
				newPlayingPlaybackState,
			);
			this.versionedMedium = new VersionedMedium(this.versionedMedium.version, newMedium);

			RegisteredClient.notify(new MediumTimeAdjusted(newMedium), this.mediumStateChangedCallbacks);
		}
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

	async insertFixedLengthMedium(name: string, lengthInMilliseconds: number): Promise<void> {
		const medium = new FixedLengthMedium(name, lengthInMilliseconds);
		const version = this.versionedMedium.version;
		await this.connection.performRequest(new InsertMediumRequest(version, medium));

		const insertedMedium = new Medium(name, lengthInMilliseconds, false, new PausedPlaybackState(0));
		if (this.versionedMedium.version > version) {
			// The medium has already been updated in the meantime during the await.
			return;
		}

		this.versionedMedium = new VersionedMedium(version + 1, insertedMedium);
	}

	async play(localStartTimeInMilliseconds: number, skipped = false): Promise<void> {
		const referenceStartTimeInMilliseconds = localStartTimeInMilliseconds + this.referenceTimeOffset;
		const playRequest = new PlayRequest(this.versionedMedium.version, skipped, referenceStartTimeInMilliseconds);
		await this.connection.performRequest(playRequest);
	}

	async pause(positionInMilliseconds: number, skipped = false): Promise<void> {
		const pauseRequest = new PauseRequest(this.versionedMedium.version, skipped, positionInMilliseconds);
		await this.connection.performRequest(pauseRequest);
	}

	async ejectMedium(): Promise<void> {
		const version = this.versionedMedium.version;
		await this.connection.performRequest(new InsertMediumRequest(this.versionedMedium.version, new EmptyMedium()));

		if (this.versionedMedium.version > version) {
			// The medium has already been updated in the meantime during the await.
			return;
		}

		this.versionedMedium = new VersionedMedium(version + 1, undefined);
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

			callbackList.splice(index, 1);
		};
	}

	logout(): void {
		this.connection.disconnect();
	}

	private connectionDidReceiveBroadcast(broadcast: BroadcastMessage): void {
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

				this.peers.splice(index, 1);

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
				const versionedMedium = VersionedMedium.fromVersionedMediumBroadcastAndReferenceTimeOffset(
					mediumStateChangedBroadcast.medium,
					this.referenceTimeOffset,
				);
				this.versionedMedium = versionedMedium;

				if (this.id === mediumStateChangedBroadcast.changed_by_id) {
					// we already know about the changes we've made and implicitly updated the state of the client.
					return;
				}

				const changer = Peer.fromMediumStateChangedBroadcast(mediumStateChangedBroadcast);
				RegisteredClient.notify(
					new MediumChangedByPeer(changer, versionedMedium.medium),
					this.mediumStateChangedCallbacks,
				);

				break;
			}
			default:
				throw new UnknownBroadcastError(broadcast);
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
		clearInterval(this.referenceTimeUpdateIntervalId);
		this.disconnectCallback(reason);
	}

	private connectionDidEncounterError(error: Event | ErrorEvent): void {
		console.error('Received error:', error);
	}
}

async function fetchReferenceTimeAndCalculateOffset(connection: Connection): Promise<number> {
	const response = await connection.performRequest(new GetReferenceTimeRequest());

	// We assume that the request takes the same time to the server as the response takes back to us.
	// Therefore, the server's reference time represents our time half way the message exchange.
	const ourTime = response.metadata.sentAt + response.metadata.roundTripTimeInMilliseconds / 2;
	const referenceTime = (response.response as ReferenceTimeMessage).milliseconds;

	return referenceTime - ourTime;
}

class UnknownBroadcastError extends Error {
	readonly broadcast: BroadcastMessage;

	constructor(broadcast: BroadcastMessage) {
		super(`Unknown broadcast received: ${broadcast.type}.`);

		this.name = UnknownBroadcastError.name;
		this.broadcast = broadcast;
	}
}

export type DisconnectCallback = (reason: CloseReason) => void;
export type PeerLifecycleCallback = (peerChange: PeerLifecycleMessage) => void;
export type ChatMessageCallback = (message: ChatMessage) => void;
export type MediumStateChangedCallback = (change: MediumChangedByPeer | MediumTimeAdjusted) => void;

export type Unsubscriber = () => void;
