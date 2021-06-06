import {BroadcastCallback, Connection, UnassignableResponseCallback, WebSocketConnection} from '$client/connection';

export interface Transport {
	connect(
		broadcastCallback: BroadcastCallback,
		unassignableResponseCallback: UnassignableResponseCallback
	): Promise<Connection>
}

export class WebSocketTransport implements Transport {
	readonly endpoint: string;

	constructor(endpoint: string) {
		this.endpoint = endpoint;
	}

	async connect(
		broadcastCallback: BroadcastCallback,
		unassignableResponseCallback: UnassignableResponseCallback
	): Promise<Connection> {
		const webSocket = await new Promise<WebSocket>((resolve, reject) => {
			const webSocket = new WebSocket(this.endpoint);
			webSocket.onopen = (event) => {
				console.log('Opening WebSocket succeeded:', event);
				resolve(webSocket);
			};
			webSocket.onerror = (event) => {
				console.log('Failed to open WebSocket:', event);
				reject();
			};
		});

		return new WebSocketConnection(
			webSocket,
			broadcastCallback,
			unassignableResponseCallback
		);
	}
}
