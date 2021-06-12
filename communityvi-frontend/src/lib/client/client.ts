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

export class RegisteredClient implements ConnectionDelegate {
	readonly id: number;
	readonly name: string;

	private readonly connection: Connection;

	constructor(id: number, name: string, connection: Connection) {
		this.id = id;
		this.name = name;
		this.connection = connection;

		this.connection.setDelegate(this);
	}

	logout(): void {
		this.connection.disconnect();
	}

	connectionDidReceiveBroadcast(broadcast: ServerResponse): void {
		console.info('Received broadcast:', broadcast);
	}

	connectionDidReceiveUnassignableResponse(response: ServerResponse): void {
		console.warn('Received unassignable response:', response);
	}

	connectionDidClose(): void {
		console.warn('Connection closed.');
	}
}
