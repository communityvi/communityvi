import type {Transport} from '$lib/client/transport';
import {WebSocketTransport} from '$lib/client/transport';
import type {Connection} from '$lib/client/connection';
import {mock} from 'jest-mock-extended';
import {
	ClientResponse,
	HelloMessage,
	ReferenceTimeMessage,
	SuccessMessageType,
	VersionedMediumResponse,
} from '$lib/client/response';
import {GetReferenceTimeRequest, MediumType, RegisterRequest} from '$lib/client/request';
import {Peer} from '$lib/client/model';
import {EnrichedResponse, ResponseMetadata} from '$lib/client/connection';

export default class TestTransport implements Transport {
	private id = 0;
	private readonly peers = new Array<Peer>();

	private readonly webSocketTransport?: WebSocketTransport = undefined;

	constructor() {
		const endpoint = process.env.COMMUNITYVI_TEST_WS_ENDPOINT;
		if (endpoint !== undefined) {
			const trimmedEndpoint = endpoint.trim();
			console.info(`[REAL] Running with real Transport at: '${trimmedEndpoint}'`);
			this.webSocketTransport = new WebSocketTransport(new URL(trimmedEndpoint));
		} else {
			console.warn('[MOCK] Running with mocked Transport!');
		}
	}

	connect(): Promise<Connection> {
		if (!this.webSocketTransport) {
			return Promise.resolve(this.mockedConnection());
		}

		return this.webSocketTransport.connect();
	}

	private mockedConnection(): Connection {
		const mockedConnection = mock<Connection>();

		const clients = this.peers.map(peer => <ClientResponse>{id: peer.id, name: peer.name});

		const helloMessage = <HelloMessage>{
			type: SuccessMessageType.Hello,
			id: ++this.id,
			clients,
			current_medium: <VersionedMediumResponse>{
				type: MediumType.Empty,
				version: 0,
			},
		};
		const helloMessageMetadata = new ResponseMetadata(performance.now(), performance.now() + 1);
		const helloResponse = new EnrichedResponse(helloMessage, helloMessageMetadata);

		const referenceTimeMessage = <ReferenceTimeMessage>{
			type: SuccessMessageType.ReferenceTime,
			milliseconds: performance.now() - 1,
		};
		const referenceTimeMessageMetadata = new ResponseMetadata(performance.now(), performance.now() + 1);
		const referenceTimeResponse = new EnrichedResponse(referenceTimeMessage, referenceTimeMessageMetadata);

		mockedConnection.performRequest.mockImplementation(request => {
			if (request instanceof RegisterRequest) {
				return Promise.resolve(helloResponse);
			}

			if (request instanceof GetReferenceTimeRequest) {
				return Promise.resolve(referenceTimeResponse);
			}

			return Promise.reject(`Don't know how to mock request: '${request.type}'`);
		});
		const peer = new Peer(this.id, `Client: #${this.id}`);
		this.peers.push(peer);

		return mockedConnection;
	}
}
