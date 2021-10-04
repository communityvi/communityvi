import {Medium, PausedPlaybackState, PlayingPlaybackState} from '$lib/client/model';

describe('Medium', () => {
	describe('metadata change detection', () => {
		const birdman = new Medium('Birdman', 116 * 60 * 1000);

		it('compares two empty mediums', () => {
			const emptyMedium = undefined;
			const anotherEmptyMedium = undefined;

			const comparison = Medium.hasChangedMetadata(emptyMedium, anotherEmptyMedium);

			expect(comparison).toBe(false);
		});

		it('compares an empty with a non-empty medium', () => {
			const emptyMedium = undefined;

			const emptyFirstComparison = Medium.hasChangedMetadata(emptyMedium, birdman);
			const emptySecondComparison = Medium.hasChangedMetadata(birdman, emptyMedium);

			expect(emptyFirstComparison).toBe(true);
			expect(emptySecondComparison).toBe(true);
			expect(emptyFirstComparison).toBe(emptySecondComparison);
		});

		it('compares two different non-empty mediums', () => {
			const wargames = new Medium('WarGames', 114 * 60 * 1000);

			const birdmanWargamesComparison = Medium.hasChangedMetadata(birdman, wargames);
			const wargamesBirdmanComparison = Medium.hasChangedMetadata(wargames, birdman);

			expect(birdmanWargamesComparison).toBe(true);
			expect(wargamesBirdmanComparison).toBe(true);
			expect(birdmanWargamesComparison).toBe(wargamesBirdmanComparison);
		});

		it('compares identical non-empty mediums', () => {
			const comparison = Medium.hasChangedMetadata(birdman, birdman);

			expect(comparison).toBe(false);
		});

		it('compares only metadata', () => {
			const playingBirdman = new Medium('Birdman', 116 * 60 * 1000, true, new PlayingPlaybackState(1337));
			const pausedBirdman = new Medium('Birdman', 116 * 60 * 1000, false, new PausedPlaybackState(42));

			const comparison = Medium.hasChangedMetadata(playingBirdman, pausedBirdman);

			expect(comparison).toBe(false);
		});

		it('compares name and length', () => {
			const bidman = new Medium('Bidman', birdman.lengthInMilliseconds);
			const birdmanTrailer = new Medium('Birdman', 2 * 60 * 1000);

			const differentLengthComparison = Medium.hasChangedMetadata(birdman, birdmanTrailer);
			const differentNameComparison = Medium.hasChangedMetadata(birdman, bidman);

			expect(differentLengthComparison).toBe(true);
			expect(differentNameComparison).toBe(true);
		});
	});
});
