import type {ErrorResponse, SuccessMessage} from '$client/response';
import {ResponseType, ServerResponse, SuccessResponse} from '$client/response';
import type {ClientRequest, ClientRequestWithId} from '$client/request';

export class Transport {
	private readonly webSocket: WebSocket
	private readonly broadcastCallback: BroadcastCallback
	private readonly unassignableResponseCallback: UnassignableResponseCallback

	private pendingResponses: PendingResponses = {}
	private nextRequestId = 0

	static async connect(
		endpoint: string,
		broadcastCallback: BroadcastCallback,
		unassignableResponseCallback: UnassignableResponseCallback
	): Promise<Transport> {
		const webSocket = await new Promise<WebSocket>((resolve, reject) => {
			const webSocket = new WebSocket(endpoint);
			webSocket.onopen = (event) => {
				console.log('Opening WebSocket succeeded:', event);
				resolve(webSocket);
			};
			webSocket.onerror = (event) => {
				console.log('Failed to open WebSocket:', event);
				reject();
			};
		});

		return new Transport(
			webSocket,
			broadcastCallback,
			unassignableResponseCallback
		);
	}

	private constructor(
		webSocket: WebSocket,
		broadcastCallback: BroadcastCallback,
		unassignableErrorCallback: UnassignableResponseCallback
	) {
		webSocket.onmessage = (messageEvent) => {
			console.log('Received message:', messageEvent);
			const message: ServerResponse = JSON.parse(messageEvent.data);
			this.handleMessage(message, messageEvent);
		};
		this.webSocket = webSocket;
		this.broadcastCallback = broadcastCallback;
		this.unassignableResponseCallback = unassignableErrorCallback;
	}

	private handleMessage(serverResponse: ServerResponse, event: MessageEvent): void {
		switch (serverResponse.type) {
		case ResponseType.Success: {
			console.log('Success received:', serverResponse);
			const successResponse = serverResponse as SuccessResponse;

			const pendingResponse = this.takePendingResponse(successResponse.request_id);
			if (!pendingResponse) {
				this.unassignableResponseCallback(successResponse);
				break;
			}

			pendingResponse.resolve(successResponse.message);
			break;
		}
		case ResponseType.Error: {
			console.log('Error received:', serverResponse);
			const errorResponse = serverResponse as ErrorResponse;

			const pendingResponse = this.takePendingResponse(errorResponse.request_id);
			if (!pendingResponse) {
				this.unassignableResponseCallback(errorResponse);
				break;
			}

			pendingResponse.reject(errorResponse);
			break;
		}
		case ResponseType.Broadcast:
			console.log('Broadcast received:', serverResponse);
			this.broadcastCallback(serverResponse);
			break;
		}
	}

	private takePendingResponse(requestId?: number): PendingResponse | undefined {
		if (!requestId) {
			return undefined;
		}

		const pendingResponse = this.pendingResponses[requestId];
		delete this.pendingResponses[requestId];

		return pendingResponse;
	}

	performRequest(request: ClientRequest): Promise<SuccessMessage> {
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

		this.webSocket.send(JSON.stringify(requestWithId));

		return pending;
	}
}

type BroadcastCallback = (broadcast: ServerResponse) => void
type UnassignableResponseCallback = (response: ServerResponse) => void

type PendingResponses = Record<number, PendingResponse>

interface PendingResponse {
	readonly requestType: string
	readonly resolve: (message: SuccessMessage) => void
	readonly reject: (error: ErrorResponse) => void
}
