import type {ServerResponse} from '$lib/client/response';
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
	InsertMediumRequest,
	PauseRequest,
	PlayRequest,
} from '$lib/client/request';
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
	PlayingPlaybackState,
	MediumChangedByOurself,
	MediumStateChanged,
	PeersRefreshedMessage,
} from '$lib/client/model';
import MessageBroker, {Subscriber, Unsubscriber} from '$lib/client/message_broker';
import type ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';
import {RESTClient} from '$lib/client/RESTClient';

export default class RegisteredClient {
	id?: number;
	readonly name: string;
	private referenceTimeSynchronizer: ReferenceTimeSynchronizer;
	private versionedMedium: VersionedMedium;
	readonly peers: Array<Peer>;

	private readonly restClient: RESTClient;

	private readonly connection: Connection;
	private readonly disconnectCallback: DisconnectCallback;

	private readonly peerLifecycleMessageBroker = new MessageBroker<PeerLifecycleMessage>();
	private readonly chatMessageBroker = new MessageBroker<ChatMessage>();
	private readonly mediumStateChangedMessageBroker = new MessageBroker<MediumStateChanged>();

	get currentMedium(): Medium | undefined {
		return this.versionedMedium.medium;
	}

	constructor(
		name: string,
		referenceTimeSynchronizer: ReferenceTimeSynchronizer,
		versionedMedium: VersionedMedium,
		restClient: RESTClient,
		connection: Connection,
		disconnectCallback: DisconnectCallback,
	) {
		this.name = name;
		this.referenceTimeSynchronizer = referenceTimeSynchronizer;
		this.versionedMedium = versionedMedium;
		this.peers = new Array<Peer>();

		this.restClient = restClient;

		this.connection = connection;
		this.disconnectCallback = disconnectCallback;

		this.connection.setDelegate({
			connectionDidReceiveBroadcast: response => this.connectionDidReceiveBroadcast(response),
			connectionDidReceiveUnassignableResponse: RegisteredClient.connectionDidReceiveUnassignableResponse,
			connectionDidClose: reason => this.connectionDidClose(reason),
			connectionDidEncounterError: RegisteredClient.connectionDidEncounterError,
		});

		this.referenceTimeSynchronizer.start((referenceTimeDeltaInMilliseconds: number) =>
			this.referenceTimeUpdated(referenceTimeDeltaInMilliseconds),
		);
	}

	private referenceTimeUpdated(referenceTimeDeltaInMilliseconds: number): void {
		const medium = this.versionedMedium.medium;
		const playbackState = medium?.playbackState;
		if (medium === undefined || !(playbackState instanceof PlayingPlaybackState)) {
			return;
		}

		const newStartTime = playbackState.localStartTimeInMilliseconds + referenceTimeDeltaInMilliseconds;
		const newPlayingPlaybackState = new PlayingPlaybackState(newStartTime);
		const newMedium = new Medium(
			medium.name,
			medium.lengthInMilliseconds,
			medium.playbackSkipped,
			newPlayingPlaybackState,
		);
		this.versionedMedium = new VersionedMedium(this.versionedMedium.version, newMedium);

		this.mediumStateChangedMessageBroker.notify(
			new MediumTimeAdjusted(newMedium, referenceTimeDeltaInMilliseconds),
		);
	}

	asPeer(): Peer {
		// FIXME
		// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
		return new Peer(this.id!, this.name);
	}

	subscribeToPeerChanges(subscriber: Subscriber<PeerLifecycleMessage>): Unsubscriber {
		return this.peerLifecycleMessageBroker.subscribe(subscriber);
	}

	async sendChatMessage(message: string): Promise<void> {
		await this.connection.performRequest(new ChatRequest(message));
	}

	subscribeToChatMessages(subscriber: Subscriber<ChatMessage>): Unsubscriber {
		return this.chatMessageBroker.subscribe(subscriber);
	}

	async insertFixedLengthMedium(name: string, lengthInMilliseconds: number): Promise<void> {
		const medium = new FixedLengthMedium(name, lengthInMilliseconds);
		const version = this.versionedMedium.version;
		await this.connection.performRequest(new InsertMediumRequest(version, medium));

		const insertedMedium = new Medium(name, lengthInMilliseconds);
		if (this.versionedMedium.version > version) {
			// The medium has already been updated in the meantime during the await.
			return;
		}

		this.versionedMedium = new VersionedMedium(version + 1, insertedMedium);

		this.mediumStateChangedMessageBroker.notify(new MediumChangedByOurself(insertedMedium));
	}

	async play(localStartTimeInMilliseconds: number, skipped = false): Promise<void> {
		const startTimeInMs =
			this.referenceTimeSynchronizer.calculateServerTimeFromLocalTime(localStartTimeInMilliseconds);
		const playRequest = new PlayRequest(this.versionedMedium.version, skipped, startTimeInMs);
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

		this.mediumStateChangedMessageBroker.notify(new MediumChangedByOurself(undefined));
	}

	subscribeToMediumStateChanges(subscriber: Subscriber<MediumStateChanged>): Unsubscriber {
		return this.mediumStateChangedMessageBroker.subscribe(subscriber);
	}

	logout(): void {
		this.connection.disconnect();
	}

	private connectionDidReceiveBroadcast(broadcast: BroadcastMessage): void {
		switch (broadcast.type) {
			case BroadcastType.ClientJoined:
				this.handleClientJoinedBroadcast(broadcast as ClientJoinedBroadcast);
				break;

			case BroadcastType.ClientLeft:
				this.handleClientLeftBroadcast(broadcast as ClientLeftBroadcast);
				break;

			case BroadcastType.Chat:
				this.handleChatBroadcast(broadcast as ChatBroadcast);
				break;

			case BroadcastType.MediumStateChanged:
				this.handleMediumStateChangedBroadcast(broadcast as MediumStateChangedBroadcast);
				break;

			default:
				throw new UnknownBroadcastError(broadcast);
		}
	}

	private handleClientJoinedBroadcast(clientJoinedBroadcast: ClientJoinedBroadcast) {
		// FIXME: We joined, this is a bogus change...
		if (this.id === undefined && clientJoinedBroadcast.name === this.name) {
			this.id = clientJoinedBroadcast.id;
			this.peers.splice(0, this.peers.length, ...clientJoinedBroadcast.participants
				.map(Peer.fromParticipant)
				.filter(participant => !participant.equals(this.asPeer())));
			this.peerLifecycleMessageBroker.notify(new PeersRefreshedMessage(this.peers));
			return;
		}

		this.peers.push(Peer.fromClientBroadcast(clientJoinedBroadcast));

		const peerLifecycleMessage = PeerJoinedMessage.fromClientJoinedBroadcast(clientJoinedBroadcast);
		this.peerLifecycleMessageBroker.notify(peerLifecycleMessage);
	}

	private handleClientLeftBroadcast(clientLeftBroadcast: ClientLeftBroadcast) {
		const index = this.peers.findIndex(peer => peer.id === clientLeftBroadcast.id);
		if (index === -1) {
			console.error('Unknown peer left:', clientLeftBroadcast);
			return;
		}

		this.peers.splice(index, 1);

		const peerLifecycleMessage = PeerLeftMessage.fromClientLeftBroadcast(clientLeftBroadcast);
		this.peerLifecycleMessageBroker.notify(peerLifecycleMessage);
	}

	private handleChatBroadcast(chatBroadcast: ChatBroadcast) {
		if (this.id === chatBroadcast.sender_id) {
			// we already know what message we've sent ourselves
			return;
		}

		const chatMessage = ChatMessage.fromChatBroadcast(chatBroadcast);
		this.chatMessageBroker.notify(chatMessage);
	}

	private handleMediumStateChangedBroadcast(mediumStateChangedBroadcast: MediumStateChangedBroadcast) {
		const versionedMedium = VersionedMedium.fromVersionedMediumBroadcastAndReferenceTimeOffset(
			mediumStateChangedBroadcast.medium,
			this.referenceTimeSynchronizer.offset,
		);
		this.versionedMedium = versionedMedium;

		if (this.id === mediumStateChangedBroadcast.changed_by_id) {
			// we already know about the changes we've made and implicitly updated the state of the client.
			return;
		}

		const changer = Peer.fromMediumStateChangedBroadcast(mediumStateChangedBroadcast);
		this.mediumStateChangedMessageBroker.notify(new MediumChangedByPeer(changer, versionedMedium.medium));
	}

	private static connectionDidReceiveUnassignableResponse(response: ServerResponse): void {
		console.warn('Received unassignable response:', response);
	}

	private connectionDidClose(reason: CloseReason): void {
		this.referenceTimeSynchronizer.stop();
		this.disconnectCallback(reason);
	}

	private static connectionDidEncounterError(error: Event | ErrorEvent): void {
		console.error('Received error:', error);
	}
}

export class UnknownBroadcastError extends Error {
	readonly broadcast: BroadcastMessage;

	constructor(broadcast: BroadcastMessage) {
		super(`Unknown broadcast received: ${broadcast.type}.`);

		this.name = UnknownBroadcastError.name;
		this.broadcast = broadcast;
	}
}

export type DisconnectCallback = (reason: CloseReason) => void;
