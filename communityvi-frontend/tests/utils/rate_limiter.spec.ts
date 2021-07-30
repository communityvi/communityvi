import RateLimiter from '$lib/utils/rate_limiter';
import TimeMock from '../client/helper/time_mock';

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

	it('should delay an immediate second call', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const rateLimiter = new RateLimiter(INTERVAL);
			const call = jest.fn();

			rateLimiter.call(call);
			rateLimiter.call(call);
			await timeMock.advanceTimeByMilliseconds(INTERVAL);

			expect(call).toHaveBeenCalledTimes(2);
		});
	});

	it('should call again immediately after enough time has passed', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const rateLimiter = new RateLimiter(INTERVAL);
			const call = jest.fn();

			rateLimiter.call(call);
			await timeMock.advanceTimeByMilliseconds(INTERVAL);
			rateLimiter.call(call);

			expect(call).toHaveBeenCalledTimes(2);
		});
	});

	it('should always call the last call', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const call = jest.fn();
			const lastCall = jest.fn();
			const rateLimiter = new RateLimiter(INTERVAL);

			rateLimiter.call(call);
			rateLimiter.call(call);
			rateLimiter.call(lastCall);
			await timeMock.advanceTimeByMilliseconds(INTERVAL);

			expect(call).toHaveBeenCalledTimes(1);
			expect(lastCall).toHaveBeenCalledTimes(1);
		});
	});
});
