/**
 * Test helper for mocking time.
 *
 * Regular time mocking by jest (https://jestjs.io/docs/timer-mocks) has a problem with scheduling promises in timers.
 *
 * Namely when scheduling asynchronous functions in setTimeout or setInterval, jest.advanceTimersByTime does not
 * guarantee that the function has been run completely.
 */
export default class TimeMock {
	private readonly realSetTimeout: typeof setTimeout;

	static async run(test: (timeMock: TimeMock) => Promise<void>): Promise<void> {
		const timeMock = new TimeMock();
		try {
			await test(timeMock);
		} finally {
			TimeMock.reset();
		}
	}

	private constructor() {
		this.realSetTimeout = setTimeout;

		jest.useFakeTimers();
	}

	private static reset(): void {
		jest.clearAllTimers();
		jest.useRealTimers();
	}

	async advanceTimeByMilliseconds(milliseconds: number): Promise<void> {
		jest.advanceTimersByTime(milliseconds);
		await this.flushPromises();
	}

	// See: https://stackoverflow.com/questions/52177631/jest-timer-and-promise-dont-work-well-settimeout-and-async-function
	// https://github.com/sinonjs/fake-timers/issues/114#issuecomment-777238105
	// and https://github.com/kentor/flush-promises/blob/f33ac564190c784019f1f689dd544187f4b77eb2/index.js
	//
	// The setTimeout(resolve, 0) should put the callback at the end of the event queue, behind any currently scheduled
	// Promises, therefore hopefully resolving only once all previous promises have run.
	//
	// Note that setImmediate isn't available anymore since Jest 27
	private flushPromises(): Promise<void> {
		return new Promise(resolve => this.realSetTimeout(resolve, 0));
	}
}
