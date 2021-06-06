import type {
	HelloMessage,
	ServerResponse,
} from './response';
import {RegisterRequest} from '$client/request';
import {Transport} from '$client/transport';

export class Client {
	readonly endpoint: string;

	constructor(endpoint: string) {
		this.endpoint = endpoint;
	}

	async register(name: string): Promise<RegisteredClient> {
		const transport = await Transport.connect(this.endpoint, Client.log, Client.log);
		const response = await transport.performRequest(new RegisterRequest(name)) as HelloMessage;

		return new RegisteredClient(response.id, name, transport);
	}

	private static log(response: ServerResponse) {
		console.log(response);
	}
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;

	private readonly transport: Transport;

	constructor(id: number, name: string, transport: Transport) {
		this.id = id;
		this.name = name;
		this.transport = transport;
	}
}
