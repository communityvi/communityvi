import {Connection, WebSocketConnection} from '$lib/client/connection';

export interface Transport {
	connect(): Promise<Connection>;
}

export class WebSocketTransport implements Transport {
	readonly endpoint: URL;
	readonly timeoutInMilliseconds: number;

	constructor(endpoint: URL, timeoutInMilliseconds = 5_000) {
		this.endpoint = endpoint;
		this.timeoutInMilliseconds = timeoutInMilliseconds;
	}

	async connect(): Promise<Connection> {
		const webSocket = await new Promise<WebSocket>((resolve, reject) => {
			const webSocket = new WebSocket(this.endpoint.toString());
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

export class ConnectionFailedError extends Error {
	readonly endpoint: URL;
	readonly cause: Event | ErrorEvent;

	constructor(endpoint: URL, cause: Event | ErrorEvent) {
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
