import type {HelloMessage, ServerResponse} from '$lib/client/response';
import {RegisterRequest} from '$lib/client/request';
import type {Transport} from '$lib/client/transport';
import type {Connection, ConnectionDelegate} from '$lib/client/connection';

export class Client {
	readonly transport: Transport;

	constructor(transport: Transport) {
		this.transport = transport;
	}

	async register(name: string): Promise<RegisteredClient> {
		const connection = await this.transport.connect();

		const response = (await connection.performRequest(new RegisterRequest(name))) as HelloMessage;

		return new RegisteredClient(response.id, name, connection);
	}
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;

	private readonly connectionDelegate: ConnectionDelegate = {
		connectionDidReceiveBroadcast: this.connectionDidReceiveBroadcast,
		connectionDidReceiveUnassignableResponse: this.connectionDidReceiveUnassignableResponse,
		connectionDidClose: this.connectionDidClose,
		connectionDidEncounterError: this.connectionDidEncounterError,
	};

	private readonly connection: Connection;

	constructor(id: number, name: string, connection: Connection) {
		this.id = id;
		this.name = name;
		this.connection = connection;

		this.connection.setDelegate(this.connectionDelegate);
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
		console.warn('Connection closed.');
	}

	private connectionDidEncounterError(error: Event | ErrorEvent): void {
		console.error('Received error:', error);
	}
}
