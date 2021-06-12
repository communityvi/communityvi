export interface HelloMessage extends SuccessMessage {
	readonly id: number;
	readonly clients: Array<number>;
	// FIXME: Left out for now, needs `VersionedMedium` type.
	//readonly current_medium:
}

export interface ReferenceTimeMessage extends SuccessMessage {
	readonly milliseconds: number;
}

export interface TimestampedSuccessMessage extends SuccessMessage {
	readonly arrivalTimestamp: DOMHighResTimeStamp | DOMTimeStamp;
}

export interface SuccessMessage {
	readonly type: SuccessMessageType;
}

export enum SuccessMessageType {
	Hello = 'hello',
	ReferenceTime = 'reference_time',
	Success = 'success',
}

export interface SuccessResponse extends ServerResponse {
	readonly request_id: number;
	readonly message: SuccessMessage;
}

export interface ErrorResponse extends ServerResponse {
	readonly request_id?: number;
	readonly message: ErrorMessage;
}

export class ResponseError extends Error {
	readonly error: ErrorMessage;

	constructor(errorResponse: ErrorResponse) {
		super(`[${errorResponse.message.error}] '${errorResponse.message.message}'`);

		this.name = ResponseError.name;
		this.error = errorResponse.message;
	}
}

export interface ErrorMessage {
	readonly error: ErrorMessageType;
	readonly message: string;
}

enum ErrorMessageType {
	InvalidFormat = 'invalid_format',
	InvalidOperation = 'invalid_operation',
	InternalServerError = 'internal_server_error',
	IncorrectMediumVersion = 'incorrect_medium_version',
	EmptyChatMessage = 'empty_chat_message',
}

export interface ServerResponse {
	readonly type: ResponseType;
}

export enum ResponseType {
	Success = 'success',
	Error = 'error',
	Broadcast = 'broadcast',
}
