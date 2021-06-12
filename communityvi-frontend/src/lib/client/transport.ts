import {Connection, WebSocketConnection} from '$lib/client/connection';

export interface Transport {
	connect(): Promise<Connection>;
}

export class WebSocketTransport implements Transport {
	readonly endpoint: string;

	constructor(endpoint: string) {
		this.endpoint = endpoint;
	}

	async connect(): Promise<Connection> {
		const webSocket = await new Promise<WebSocket>((resolve, reject) => {
			const webSocket = new WebSocket(this.endpoint);
			webSocket.onopen = () => {
				resolve(webSocket);
			};
			webSocket.onerror = () => {
				reject();
			};
		});

		return new WebSocketConnection(webSocket);
	}
}
