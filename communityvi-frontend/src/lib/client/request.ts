export class PauseRequest implements ClientRequest {
	type = RequestType.Pause;
	readonly previous_version: number;
	readonly skipped: boolean;
	readonly position_in_milliseconds: number;

	constructor(previous_version: number, skipped: boolean, position_in_milliseconds: number) {
		this.previous_version = previous_version;
		this.skipped = skipped;
		this.position_in_milliseconds = position_in_milliseconds;
	}
}

export class PlayRequest implements ClientRequest {
	type = RequestType.Play;
	readonly previous_version: number;
	readonly skipped: boolean;
	readonly start_time_in_milliseconds: number;

	constructor(previous_version: number, skipped: boolean, start_time_in_milliseconds: number) {
		this.previous_version = previous_version;
		this.skipped = skipped;
		this.start_time_in_milliseconds = start_time_in_milliseconds;
	}
}

export class GetReferenceTimeRequest implements ClientRequest {
	type = RequestType.GetReferenceTime;
}

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

export class ChatRequest implements ClientRequest {
	type = RequestType.Chat;
	readonly message: string;

	constructor(message: string) {
		this.message = message;
	}
}

export class RegisterRequest implements ClientRequest {
	type = RequestType.Register;
	readonly name: string;

	constructor(name: string) {
		this.name = name;
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
	GetReferenceTime = 'get_reference_time',
	Play = 'play',
	Pause = 'pause',
}
