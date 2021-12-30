export class RESTClient {
	private readonly apiBaseURL: URL;

	constructor(apiBaseURL: URL) {
		this.apiBaseURL = apiBaseURL;
	}

	async getReferenceTimeMilliseconds(): Promise<number> {
		const response = await window.fetch(`${this.apiBaseURL}/reference_time_milliseconds`);

		const milliseconds = await response.json();
		if (typeof milliseconds !== 'number') {
			throw new RESTError(`Invalid reference time response: '${milliseconds}'`);
		}

		return milliseconds;
	}
}

class RESTError extends Error {
	constructor(message: string) {
		super(`RESTError: ${message}`);
	}
}
