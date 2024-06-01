import {RESTClient} from '$lib/client/RESTClient';

export default class ReferenceTimeSynchronizer {
	private readonly restClient: RESTClient;
	private callback?: TimeUpdatedCallback;
	private intervalId?: number;

	private storedOffset: number;

	static UPDATE_INTERVAL_MILLISECONDS = 15_000;

	get offset(): number {
		return this.storedOffset;
	}

	static async createInitializedWithRESTClient(restClient: RESTClient): Promise<ReferenceTimeSynchronizer> {
		const initialOffset = await ReferenceTimeSynchronizer.fetchReferenceTimeAndCalculateOffset(restClient);
		return new ReferenceTimeSynchronizer(initialOffset, restClient);
	}

	private constructor(initialOffset: number, restClient: RESTClient) {
		this.restClient = restClient;
		this.storedOffset = initialOffset;
	}

	start(callback: TimeUpdatedCallback): void {
		if (this.callback !== undefined) {
			throw new AlreadyRunningError();
		}

		this.callback = callback;

		this.intervalId = setInterval(
			() => this.synchronizeReferenceTime(),
			ReferenceTimeSynchronizer.UPDATE_INTERVAL_MILLISECONDS,
		);
	}

	private async synchronizeReferenceTime(): Promise<void> {
		const newOffset = await ReferenceTimeSynchronizer.fetchReferenceTimeAndCalculateOffset(this.restClient);
		if (this.storedOffset === newOffset) {
			console.debug('Reference time did not need updating.');
			return;
		}

		const oldOffset = this.storedOffset;
		this.storedOffset = newOffset;
		console.debug('Reference time offset updated:', this.storedOffset);

		if (this.callback !== undefined) {
			this.callback(this.storedOffset - oldOffset);
		}
	}

	private static async fetchReferenceTimeAndCalculateOffset(restClient: RESTClient): Promise<number> {
		const response = await restClient.getReferenceTimeMilliseconds();

		// We assume that the request takes the same time to the server as the response takes back to us.
		// Therefore, the server's reference time represents our time half way the message exchange.
		const ourTime = response.sentAtMilliseconds + response.roundtripTimeInMilliseconds / 2;

		return response.referenceTimeInMilliseconds - ourTime;
	}

	calculateServerTimeFromLocalTime(localTimeInMilliseconds: number): number {
		return localTimeInMilliseconds + this.storedOffset;
	}

	stop(): void {
		if (this.intervalId === undefined) {
			return;
		}

		clearInterval(this.intervalId);
		this.intervalId = undefined;
		this.callback = undefined;
	}
}

export type TimeUpdatedCallback = (referenceTimeDeltaInMilliseconds: number) => void;

export class AlreadyRunningError extends Error {
	constructor() {
		super('The synchronization process is already running!');

		this.name = AlreadyRunningError.name;
	}
}
