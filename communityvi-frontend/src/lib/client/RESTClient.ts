export class RESTClient {
	private readonly apiBaseURL: URL;

	constructor(apiBaseURL: URL) {
		this.apiBaseURL = apiBaseURL;
	}

	async getReferenceTimeMilliseconds(): Promise<ReferenceTimeResponse> {
		const response = await RESTClient.getWithTimings(`${this.apiBaseURL}/reference-time-milliseconds`);

		const milliseconds = JSON.parse(response.text);
		if (typeof milliseconds !== 'number') {
			throw new RESTError(`Invalid reference time response: '${milliseconds}'`);
		}

		return new ReferenceTimeResponse(milliseconds, response.sentAtMilliseconds, response.receivedAtMilliseconds);
	}

	private static async getWithTimings(url: string): Promise<ResponseWithTimings> {
		// Uses `XMLHttpRequest` to get more accurate timings of how long a request takes.
		// The alternative, running performance.now() before and after `await window.fetch(...)`
		// has problems with timings being off by more than 100ms sometimes.
		// This most likely comes down to promises running in the time between when the response
		// was received and the continuation after the await is run.
		// `XMLHttpRequest` has the `loadstart` event that already contains a timestamp of when
		// the event was created, so it is much more accurate although the timings can still vary
		// an order of magnitude more than with a websocket based request/response pattern.

		const request = new XMLHttpRequest();
		request.open('GET', url, true);

		let sentAtMilliseconds: number | undefined;
		let receivedAtMilliseconds: number | undefined;

		const requestPromise: Promise<string> = new Promise((resolve, reject) => {
			// This is the first event we receive that a response has arrived
			request.onloadstart = event => {
				receivedAtMilliseconds = event.timeStamp;
			};

			request.onerror = event => {
				reject(new RESTError(`Error performing get request to '${url}': '${event}'`));
			};

			request.onload = () => {
				resolve(request.responseText);
			};

			sentAtMilliseconds = performance.now();
			request.send();
		});

		const text = await requestPromise;
		if (sentAtMilliseconds === undefined || receivedAtMilliseconds === undefined) {
			throw new RESTError(`Failed to time request to '${url}'`);
		}

		return {
			text,
			sentAtMilliseconds,
			receivedAtMilliseconds,
		};
	}
}

interface ResponseWithTimings {
	text: string;
	sentAtMilliseconds: number;
	receivedAtMilliseconds: number;
}

export class ReferenceTimeResponse {
	readonly referenceTimeInMilliseconds: number;
	readonly sentAtMilliseconds: number;
	readonly receivedAtMilliseconds: number;

	constructor(referenceTimeInMilliseconds: number, sentAtMilliseconds: number, receivedAtMilliseconds: number) {
		this.referenceTimeInMilliseconds = referenceTimeInMilliseconds;
		this.sentAtMilliseconds = sentAtMilliseconds;
		this.receivedAtMilliseconds = receivedAtMilliseconds;
	}

	get roundtripTimeInMilliseconds(): number {
		return this.receivedAtMilliseconds - this.sentAtMilliseconds;
	}
}

class RESTError extends Error {
	constructor(message: string) {
		super(`RESTError: ${message}`);
	}
}
