import {Connection, WebSocketConnection} from '$lib/client/connection';

export interface Transport {
	connect(token: string): Promise<Connection>;
}

export class WebSocketTransport implements Transport {
	readonly endpoint: URL;
	readonly timeoutInMilliseconds: number;

	constructor(endpoint: URL, timeoutInMilliseconds = 5_000) {
		this.endpoint = endpoint;
		this.timeoutInMilliseconds = timeoutInMilliseconds;
	}

	async connect(token: string): Promise<Connection> {
		// Collects all messages being received before any connection delegate is registered.
		const earlyMessages: MessageEvent[] = [];
		const webSocket = await new Promise<WebSocket>((resolve, reject) => {
			const loginURL = new URL(this.endpoint.toString());
			loginURL.searchParams.set('token', token);

			const webSocket = new WebSocket(loginURL);
			webSocket.onopen = () => {
				resolve(webSocket);
				webSocket.onerror = null;
			};
			webSocket.onmessage = (message) => {
				earlyMessages.push(message);
			};
			webSocket.onerror = error => {
				reject(new ConnectionFailedError(this.endpoint, error));
				webSocket.onerror = null;
			};
		});

		return new WebSocketConnection(webSocket, this.timeoutInMilliseconds, earlyMessages);
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
