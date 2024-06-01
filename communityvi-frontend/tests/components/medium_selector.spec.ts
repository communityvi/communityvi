import {formatMediumLength} from '$lib/components/medium_selector/helpers';
import {Medium} from '$lib/client/model';
import {SelectedMedium} from '$lib/components/medium_selector/metadata_loader';
import {describe, it, expect} from 'vitest';

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

describe('SelectedMedium', () => {
	describe('recognizing meaningful differences', () => {
		const birdman = new Medium('Birdman', 116 * 60 * 1000);

		it('detects meaningful differences in name', () => {
			const bidman = new SelectedMedium('Bidman', birdman.lengthInMilliseconds);

			const isMeaningfullyDifferent = bidman.isMeaningfullyDifferentTo(birdman);

			expect(isMeaningfullyDifferent).toBe(true);
		});

		it('does not detect whitespace naming differences', () => {
			const birdmanWithWhitespace = new SelectedMedium(' Birdman ', birdman.lengthInMilliseconds);

			const isMeaningfullyDifferent = birdmanWithWhitespace.isMeaningfullyDifferentTo(birdman);

			expect(isMeaningfullyDifferent).toBe(false);
		});

		it('detects meaningful differences in length', () => {
			const birdmanDirectorsCut = new SelectedMedium('Birdman', birdman.lengthInMilliseconds + 1_000);

			const isMeaningfullyDifferent = birdmanDirectorsCut.isMeaningfullyDifferentTo(birdman);

			expect(isMeaningfullyDifferent).toBe(true);
		});

		it('does not detects differences within one second', () => {
			const selectedBirdman = new SelectedMedium('Birdman', birdman.lengthInMilliseconds + 420);

			const isMeaningfullyDifferent = selectedBirdman.isMeaningfullyDifferentTo(birdman);

			expect(isMeaningfullyDifferent).toBe(false);
		});
	});
});
