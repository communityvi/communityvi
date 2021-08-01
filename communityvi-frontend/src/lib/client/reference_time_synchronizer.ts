import type {Connection} from '$lib/client/connection';
import {GetReferenceTimeRequest} from '$lib/client/request';
import type {ReferenceTimeMessage} from '$lib/client/response';

export default class ReferenceTimeSynchronizer {
	private readonly connection: Connection;
	private callback?: TimeUpdatedCallback;
	private intervalId?: NodeJS.Timeout;

	private storedOffset: number;

	get offset(): number {
		return this.storedOffset;
	}

	static async createInitializedWithConnection(connection: Connection): Promise<ReferenceTimeSynchronizer> {
		const initialOffset = await ReferenceTimeSynchronizer.fetchReferenceTimeAndCalculateOffset(connection);
		return new ReferenceTimeSynchronizer(initialOffset, connection);
	}

	private constructor(initialOffset: number, connection: Connection) {
		this.storedOffset = initialOffset;
		this.connection = connection;
	}

	start(callback: TimeUpdatedCallback): void {
		if (this.callback !== undefined) {
			throw new AlreadyRunningError();
		}

		this.callback = callback;

		// Schedule reference time updates every 15s
		this.intervalId = setInterval(() => this.synchronizeReferenceTime(), 15_000);
	}

	private async synchronizeReferenceTime(): Promise<void> {
		const newOffset = await ReferenceTimeSynchronizer.fetchReferenceTimeAndCalculateOffset(this.connection);
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

	private static async fetchReferenceTimeAndCalculateOffset(connection: Connection): Promise<number> {
		const response = await connection.performRequest(new GetReferenceTimeRequest());

		// We assume that the request takes the same time to the server as the response takes back to us.
		// Therefore, the server's reference time represents our time half way the message exchange.
		const ourTime = response.metadata.sentAt + response.metadata.roundTripTimeInMilliseconds / 2;
		const referenceTime = (response.response as ReferenceTimeMessage).milliseconds;

		return referenceTime - ourTime;
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
