import RateLimiter from '$lib/utils/rate_limiter';

describe('The RateLimiter', () => {
	const INTERVAL = 1000;

	it('should immediately perform a single call', () => {
		const rateLimiter = new RateLimiter(INTERVAL);
		const call = jest.fn();

		rateLimiter.call(call);

		expect(call).toHaveBeenCalledTimes(1);
	});

	it('should not immediately perform a second call', () => {
		const rateLimiter = new RateLimiter(INTERVAL);
		const call = jest.fn();

		rateLimiter.call(call);
		rateLimiter.call(call);

		expect(call).toHaveBeenCalledTimes(1);
	});

	it('should delay an immediate second call', () => {
		jest.useFakeTimers();
		const rateLimiter = new RateLimiter(INTERVAL);
		const call = jest.fn();

		rateLimiter.call(call);
		rateLimiter.call(call);
		jest.advanceTimersByTime(INTERVAL);

		expect(call).toHaveBeenCalledTimes(2);
	});

	it('should call again immediately after enough time has passed', () => {
		const originalNow = performance.now;
		try {
			// given
			jest.useFakeTimers();

			let time = 0;
			performance.now = jest.fn(() => time);

			const rateLimiter = new RateLimiter(INTERVAL);
			const call = jest.fn();

			// when
			rateLimiter.call(call);

			time += INTERVAL;
			jest.advanceTimersByTime(INTERVAL);

			rateLimiter.call(call);

			// then
			expect(call).toHaveBeenCalledTimes(2);
		} finally {
			performance.now = originalNow;
		}
	});

	it('should always call the last call', () => {
		jest.useFakeTimers();
		const call = jest.fn();
		const lastCall = jest.fn();
		const rateLimiter = new RateLimiter(INTERVAL);

		rateLimiter.call(call);
		rateLimiter.call(call);
		rateLimiter.call(lastCall);
		jest.advanceTimersByTime(INTERVAL);

		expect(call).toHaveBeenCalledTimes(1);
		expect(lastCall).toHaveBeenCalledTimes(1);
	});
});
