import type {Transport} from '$lib/client/transport';
import {WebSocketTransport} from '$lib/client/transport';
import type {Connection} from '$lib/client/connection';
import {mock} from 'jest-mock-extended';
import {HelloMessage, MediumResponse, SuccessMessageType, VersionedMediumResponse} from '$lib/client/response';
import {MediumType} from '$lib/client/request';

export default class TestTransport implements Transport {
	private id = 0;
	private readonly clients = new Array<number>();

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
		mockedConnection.performRequest.mockResolvedValueOnce(<HelloMessage>{
			type: SuccessMessageType.Hello,
			id: ++this.id,
			clients: [...this.clients],
			current_medium: <VersionedMediumResponse>{
				version: 0,
				medium: <MediumResponse>{
					type: MediumType.Empty,
				},
			},
		});
		this.clients.push(this.id);

		return mockedConnection;
	}
}
