import type {HelloMessage, ServerResponse} from '$lib/client/response';
import {RegisterRequest} from '$lib/client/request';
import type {Transport} from '$lib/client/transport';
import type {Connection} from '$lib/client/connection';

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

	constructor(id: number, name: string, connection: Connection, disconnectCallback: DisconnectCallback) {
		this.id = id;
		this.name = name;
		this.connection = connection;
		this.disconnectCallback = disconnectCallback;

		this.connection.setDelegate({
			connectionDidReceiveBroadcast: response => this.connectionDidReceiveBroadcast(response),
			connectionDidReceiveUnassignableResponse: response => this.connectionDidReceiveUnassignableResponse(response),
			connectionDidClose: () => this.connectionDidClose(),
			connectionDidEncounterError: error => this.connectionDidEncounterError(error),
		});
	}

	logout(): void {
		this.connection.disconnect();
	}

	private connectionDidReceiveBroadcast(broadcast: ServerResponse): void {
		console.info('Received broadcast:', broadcast);
	}

	private connectionDidReceiveUnassignableResponse(response: ServerResponse): void {
		console.warn('Received unassignable response:', response);
	}

	private connectionDidClose(): void {
		this.disconnectCallback();
	}

	private connectionDidEncounterError(error: Event | ErrorEvent): void {
		console.error('Received error:', error);
	}
}

export type DisconnectCallback = () => void;
