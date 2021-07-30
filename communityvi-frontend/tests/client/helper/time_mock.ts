/**
 * Test helper for mocking time.
 *
 * Regular time mocking by jest (https://jestjs.io/docs/26.x/timer-mocks) has two shortcomings which this
 * class fixes.
 *
 * 1. It doesn't support performance.now()
 * 2. When scheduling asynchronous functions in setTimeout or setInterval, jest.advanceTimersByTime does
 *    not guarantee that the function has been run completely.
 */
export default class TimeMock {
	private readonly originalPerformanceNow: () => DOMTimeStamp | DOMHighResTimeStamp;
	private nowInMilliseconds: number;
	private readonly performanceNow = jest.fn(() => this.nowInMilliseconds);

	static async run(test: (timeMock: TimeMock) => Promise<void>, initialNowInMilliseconds = 0): Promise<void> {
		const timeMock = new TimeMock(initialNowInMilliseconds);
		try {
			await test(timeMock);
		} finally {
			timeMock.reset();
		}
	}

	private constructor(nowInMilliseconds: number) {
		this.originalPerformanceNow = performance.now;
		this.nowInMilliseconds = nowInMilliseconds;
		performance.now = this.performanceNow;

		jest.useFakeTimers();
	}

	private reset(): void {
		jest.clearAllTimers();
		jest.useRealTimers();
		performance.now = this.originalPerformanceNow;
	}

	async advanceTimeByMilliseconds(milliseconds: number): Promise<void> {
		this.nowInMilliseconds += milliseconds;
		jest.advanceTimersByTime(milliseconds);
		await TimeMock.flushPromises();
	}

	// See: https://stackoverflow.com/questions/52177631/jest-timer-and-promise-dont-work-well-settimeout-and-async-function
	// NOTE: This is only intended for use in NodeJS, which is the case in the test cases.
	private static flushPromises(): Promise<void> {
		return new Promise(resolve => setImmediate(resolve));
	}
}
