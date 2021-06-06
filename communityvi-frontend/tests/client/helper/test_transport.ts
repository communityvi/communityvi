import type {Transport} from '$client/transport';
import type {BroadcastCallback, Connection, UnassignableResponseCallback, ClosedCallback} from '$client/connection';
import {mock} from 'jest-mock-extended';
import {HelloMessage, SuccessMessageType} from '$client/response';
import {WebSocketTransport} from '$client/transport';

export default class TestTransport implements Transport {
	private id = 0;
	private readonly clients = new Array<number>();

	private readonly webSocketTransport?: WebSocketTransport = undefined;

	constructor() {
		const endpoint = process.env.COMMUNITYVI_TEST_WS_ENDPOINT;
		if (endpoint !== undefined) {
			const trimmedEndpoint = endpoint.trim();
			console.info(`[REAL] Running with real Transport at: '${trimmedEndpoint}'`);
			this.webSocketTransport = new WebSocketTransport(trimmedEndpoint);
		} else {
			console.warn('[MOCK] Running with mocked Transport!');
		}
	}

	connect(
		broadcastCallback: BroadcastCallback,
		unassignableResponseCallback: UnassignableResponseCallback,
		closedCallback: ClosedCallback,
	): Promise<Connection> {
		if (!this.webSocketTransport) {
			return Promise.resolve(this.mockedConnection());
		}

		return this.webSocketTransport.connect(broadcastCallback, unassignableResponseCallback, closedCallback);
	}

	private mockedConnection(): Connection {
		const mockedConnection = mock<Connection>();
		mockedConnection.performRequest.mockResolvedValueOnce(<HelloMessage>{
			type: SuccessMessageType.Hello,
			id: ++this.id,
			clients: [...this.clients],
		});
		this.clients.push(this.id);

		return mockedConnection;
	}
}
