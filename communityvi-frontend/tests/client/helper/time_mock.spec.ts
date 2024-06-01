import TimeMock from './time_mock';
import {describe, it, expect} from 'vitest';

describe('TimeMock', () => {
	it('should mock performance timer', async () => {
		await TimeMock.run(async timeMock => {
			const first = performance.now();
			await timeMock.advanceTimeByMilliseconds(42);
			const second = performance.now();

			expect(second - first).toStrictEqual(42);
		});
	});
});
