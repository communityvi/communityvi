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
}
