import {Connection, WebSocketConnection} from '$lib/client/connection';

export interface Transport {
	connect(): Promise<Connection>;
}

export class WebSocketTransport implements Transport {
	readonly endpoint: string;
	readonly timeoutInMilliseconds: number;

	constructor(endpoint: string, timeoutInMilliseconds = 5_000) {
		this.endpoint = endpoint;
		this.timeoutInMilliseconds = timeoutInMilliseconds;
	}

	async connect(): Promise<Connection> {
		const webSocket = await new Promise<WebSocket>((resolve, reject) => {
			const webSocket = new WebSocket(this.endpoint);
			webSocket.onopen = () => {
				resolve(webSocket);
				webSocket.onerror = null;
			};
			webSocket.onerror = error => {
				reject(new ConnectionFailedError(this.endpoint, error));
				webSocket.onerror = null;
			};
		});

		return new WebSocketConnection(webSocket, this.timeoutInMilliseconds);
	}
}

class ConnectionFailedError extends Error {
	readonly endpoint: string;
	readonly cause: Event | ErrorEvent;

	constructor(endpoint: string, cause: Event | ErrorEvent) {
		if (cause instanceof ErrorEvent) {
			super(`Could not connect to WebSocket at '${endpoint}', error was: '${cause.message}'`);
		} else {
			super(`Could not connect to WebSocket at '${endpoint}'.`);
		}

		this.name = ConnectionFailedError.name;
		this.endpoint = endpoint;
		this.cause = cause;
	}
}
