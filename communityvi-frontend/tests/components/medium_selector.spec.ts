import {formatMediumLength} from '$lib/components/medium_selector/helpers';
import {Medium} from '$lib/client/model';

describe('The MediumSelector component', () => {
	describe('medium length formatting', () => {
		const SECOND = 1000;
		const MINUTE = SECOND * 60;
		const HOUR = MINUTE * 60;
		const LOCALE = 'en-US';

		it('formats lengths', () => {
			const milliseconds = 3 * HOUR + 2 * MINUTE + 10.23 * SECOND;
			const medium = new Medium('irrelevant', milliseconds);

			const formatted = formatMediumLength(medium, LOCALE);

			expect(formatted).toBe('3h 2min 10.23s');
		});

		it('formats full hours', () => {
			const milliseconds = 2 * HOUR;
			const medium = new Medium('irrelevant', milliseconds);

			const formatted = formatMediumLength(medium, LOCALE);

			expect(formatted).toBe('2h');
		});

		it('formats full minutes', () => {
			const milliseconds = 42 * MINUTE;
			const medium = new Medium('irrelevant', milliseconds);

			const formatted = formatMediumLength(medium, LOCALE);

			expect(formatted).toBe('42min');
		});

		it('formats seconds', () => {
			const milliseconds = 13.37 * SECOND;
			const medium = new Medium('irrelevant', milliseconds);

			const formatted = formatMediumLength(medium, LOCALE);

			expect(formatted).toBe('13.37s');
		});
	});
});

export {};
