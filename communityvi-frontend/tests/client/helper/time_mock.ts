import {vi} from 'vitest';

/**
 * Test helper for mocking time.
 *
 * Regular time mocking by jest (https://jestjs.io/docs/timer-mocks) has a problem with scheduling promises in timers.
 *
 * Namely when scheduling asynchronous functions in setTimeout or setInterval, vi.advanceTimersByTime does not
 * guarantee that the function has been run completely.
 */
export default class TimeMock {
	static async run(test: (timeMock: TimeMock) => Promise<void>): Promise<void> {
		const timeMock = new TimeMock();
		try {
			await test(timeMock);
		} finally {
			TimeMock.reset();
		}
	}

	private constructor() {
		vi.useFakeTimers({
			toFake: [
				'setTimeout',
				'clearTimeout',
				'setImmediate',
				'clearImmediate',
				'setInterval',
				'clearInterval',
				'Date',
				'performance', // NOTE: This one at least (maybe others) wasn't faked by default
			],
		});
	}

	private static reset(): void {
		vi.clearAllTimers();
		vi.useRealTimers();
	}

	async advanceTimeByMilliseconds(milliseconds: number): Promise<void> {
		await vi.advanceTimersByTimeAsync(milliseconds);
	}
}
