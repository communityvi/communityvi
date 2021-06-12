import type {ClientRequest, ClientRequestWithId} from '$lib/client/request';
import {
	ErrorResponse,
	ResponseType,
	ServerResponse,
	SuccessMessage,
	SuccessResponse,
	TimestampedSuccessMessage,
} from '$lib/client/response';
import {promiseWithTimout} from '$lib/client/promises';

export interface Connection {
	setDelegate(delegate: ConnectionDelegate): void;
	performRequest(request: ClientRequest): Promise<SuccessMessage>;
	disconnect(): void;
}

export class WebSocketConnection implements Connection {
	private readonly webSocket: WebSocket;
	private readonly timeoutInMilliseconds: number;

	private delegate?: ConnectionDelegate;

	private pendingResponses: PendingResponses = {};
	private nextRequestId = 0;

	constructor(webSocket: WebSocket, timeoutInMilliseconds: number) {
		webSocket.onmessage = messageEvent => {
			const message: ServerResponse = JSON.parse(messageEvent.data);
			this.handleMessage(message, messageEvent);
		};
		webSocket.onclose = () => {
			this.delegate?.connectionDidClose();
		};

		this.webSocket = webSocket;
		this.timeoutInMilliseconds = timeoutInMilliseconds;
	}

	setDelegate(delegate: ConnectionDelegate): void {
		this.delegate = delegate;
	}

	private handleMessage(serverResponse: ServerResponse, event: MessageEvent): void {
		switch (serverResponse.type) {
			case ResponseType.Success: {
				const successResponse = serverResponse as SuccessResponse;

				const pendingResponse = this.takePendingResponse(successResponse.request_id);
				if (!pendingResponse) {
					this.delegate?.connectionDidReceiveUnassignableResponse(successResponse);
					break;
				}

				pendingResponse.resolve({arrivalTimestamp: event.timeStamp, ...successResponse.message});
				break;
			}
			case ResponseType.Error: {
				const errorResponse = serverResponse as ErrorResponse;

				const pendingResponse = this.takePendingResponse(errorResponse.request_id);
				if (!pendingResponse) {
					this.delegate?.connectionDidReceiveUnassignableResponse(errorResponse);
					break;
				}

				pendingResponse.reject(errorResponse);
				break;
			}
			case ResponseType.Broadcast: {
				this.delegate?.connectionDidReceiveBroadcast(serverResponse);
				break;
			}
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
			...request,
		} as ClientRequestWithId;

		const pending = new Promise<SuccessMessage>((resolve, reject) => {
			this.pendingResponses[requestWithId.request_id] = {
				requestType: request.type,
				resolve,
				reject,
			};
		});
		const pendingWithTimeout = promiseWithTimout(pending, this.timeoutInMilliseconds, () => {
			delete this.pendingResponses[requestWithId.request_id];
		});

		this.webSocket.send(JSON.stringify(requestWithId));

		return pendingWithTimeout;
	}

	disconnect(): void {
		// See: https://developer.mozilla.org/en-US/docs/Web/API/CloseEvent#status_codes
		const normalClosure = 1000;
		this.webSocket.close(normalClosure, 'Goodbye!');
	}
}

export interface ConnectionDelegate {
	connectionDidReceiveBroadcast(broadcast: ServerResponse): void;
	connectionDidReceiveUnassignableResponse(response: ServerResponse): void;
	connectionDidClose(): void;
}

type PendingResponses = Record<number, PendingResponse>;

interface PendingResponse {
	readonly requestType: string;
	readonly resolve: (message: TimestampedSuccessMessage) => void;
	readonly reject: (error: ErrorResponse) => void;
}
