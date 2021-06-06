import type {
	ErrorResponse,
	HelloMessage,
	ServerResponse,
	SuccessMessage,
	SuccessResponse
} from './response';
import {ResponseType} from './response';
import type {ClientRequest, ClientRequestWithId} from '$client/request';
import {RegisterRequest} from '$client/request';

export class Client {
	readonly endpoint: string;

	private openedWebSocket?: WebSocket
	private pendingResponses: PendingResponses = {}
	private nextRequestId = 0

	constructor(endpoint: string) {
		this.endpoint = endpoint;
	}

	async register(name: string): Promise<RegisteredClient> {
		this.openedWebSocket = await this.webSocketPromise();
		const response = await this.request(new RegisterRequest(name)) as HelloMessage;

		return new RegisteredClient(response.id, name, this);
	}

	private request(request: ClientRequest): Promise<SuccessMessage> {
		const requestWithId = {
			request_id: ++this.nextRequestId,
			...request
		} as ClientRequestWithId;

		const pending = new Promise<SuccessMessage>((resolve, reject) => {
			this.pendingResponses[requestWithId.request_id] = {
				requestType: request.type,
				resolve,
				reject
			};
		});

		this.openedWebSocket?.send(JSON.stringify(requestWithId));

		return pending;
	}

	private webSocketPromise(): Promise<WebSocket> {
		return new Promise<WebSocket>((resolve, reject) => {
			const webSocket = new WebSocket(this.endpoint);
			webSocket.onopen = (event) => {
				console.log('Opening WebSocket succeeded:', event);
				resolve(webSocket);
			};
			webSocket.onmessage = (messageEvent) => {
				console.log('Received message:', messageEvent);
				const message: ServerResponse = JSON.parse(messageEvent.data);
				this.handleMessage(message, messageEvent);
			};
			webSocket.onerror = (event) => {
				console.log('Failed to open WebSocket:', event);
				reject();
			};
		});
	}

	private handleMessage(serverResponse: ServerResponse, event: MessageEvent): void {
		switch (serverResponse.type) {
		case ResponseType.Success: {
			console.log('Success received:', serverResponse);
			const successResponse = serverResponse as SuccessResponse;

			const pendingResponse: PendingResponse | undefined = this.pendingResponses[successResponse.request_id];
			delete this.pendingResponses[successResponse.request_id];

			pendingResponse?.resolve(successResponse.message);
			break;
		}
		case ResponseType.Error: {
			console.log('Error received:', serverResponse);
			const errorResponse = serverResponse as ErrorResponse;
			if (!errorResponse.request_id) {
				break;
			}

			const pendingResponse: PendingResponse | undefined = this.pendingResponses[errorResponse.request_id];
			delete this.pendingResponses[errorResponse.request_id];

			pendingResponse?.reject(errorResponse);
			break;
		}
		case ResponseType.Broadcast:
			console.log('Broadcast received:', serverResponse);
			break;
		}
	}
}

type PendingResponses = Record<number, PendingResponse>

interface PendingResponse {
	readonly requestType: string
	readonly resolve: (message: SuccessMessage) => void
	readonly reject: (error: ErrorResponse) => void
}

export class RegisteredClient {
	readonly id: number;
	readonly name: string;

	private readonly client: Client;

	constructor(id: number, name: string, client: Client) {
		this.id = id;
		this.name = name;
		this.client = client;
	}
}
