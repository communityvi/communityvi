import type {HelloMessage} from '$client/response';
import {RegisterRequest} from '$client/request';
import type {Transport} from '$client/transport';
import type {Connection} from '$client/connection';

export class Client {
	readonly transport: Transport;

	constructor(transport: Transport) {
		this.transport = transport;
	}

	async register(name: string): Promise<RegisteredClient> {
		const connection = await this.transport.connect(console.log, console.warn, console.warn);

		const response = (await connection.performRequest(new RegisterRequest(name))) as HelloMessage;

		return new RegisteredClient(response.id, name, connection);
	}
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;

	private readonly connection: Connection;

	constructor(id: number, name: string, connection: Connection) {
		this.id = id;
		this.name = name;
		this.connection = connection;
	}
}
