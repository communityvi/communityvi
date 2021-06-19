export class InsertMediumRequest implements ClientRequest {
	type = RequestType.InsertMedium;
	readonly previous_version: number;
	readonly medium: Medium;

	constructor(previous_version: number, medium: Medium) {
		this.previous_version = previous_version;
		this.medium = medium;
	}
}

export class FixedLengthMedium implements Medium {
	type = MediumType.FixedLength;
	readonly name: string;
	readonly length_in_milliseconds: number;

	constructor(name: string, length_in_milliseconds: number) {
		this.name = name;
		this.length_in_milliseconds = Math.round(length_in_milliseconds);
	}
}

export class EmptyMedium implements Medium {
	type = MediumType.Empty;
}

interface Medium {
	readonly type: MediumType;
}

export enum MediumType {
	FixedLength = 'fixed_length',
	Empty = 'empty',
}

export class RegisterRequest implements ClientRequest {
	type = RequestType.Register;
	readonly name: string;

	constructor(name: string) {
		this.name = name;
	}
}

export class ChatRequest implements ClientRequest {
	type = RequestType.Chat;
	readonly message: string;

	constructor(message: string) {
		this.message = message;
	}
}

export interface ClientRequestWithId extends ClientRequest {
	readonly request_id: number;
}

export interface ClientRequest {
	readonly type: RequestType;
}

enum RequestType {
	Register = 'register',
	Chat = 'chat',
	InsertMedium = 'insert_medium',
}
