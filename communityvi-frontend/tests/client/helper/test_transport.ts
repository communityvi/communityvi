import type {Transport} from '$client/transport';
import type {BroadcastCallback, Connection, UnassignableResponseCallback} from '$client/connection';
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
			this.webSocketTransport = new WebSocketTransport(endpoint.trim());
		}
	}

	connect(
		broadcastCallback: BroadcastCallback,
		unassignableResponseCallback: UnassignableResponseCallback
	): Promise<Connection> {
		if (!this.webSocketTransport) {
			return Promise.resolve(this.mockedConnection());
		}

		return this.webSocketTransport.connect(broadcastCallback, unassignableResponseCallback);
	}

	private mockedConnection(): Connection {
		const mockedConnection = mock<Connection>();
		mockedConnection.performRequest.mockResolvedValueOnce(<HelloMessage> {
			type: SuccessMessageType.Hello,
			id: ++this.id,
			clients: [...this.clients],
		});
		this.clients.push(this.id);

		return mockedConnection;
	}
}
