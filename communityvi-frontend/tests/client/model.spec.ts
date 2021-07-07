import {Medium, PausedPlaybackState, PlayingPlaybackState} from '$lib/client/model';

describe('Medium', () => {
	describe('metadata comparison', () => {
		const birdman = new Medium('Birdman', 116 * 60 * 1000);

		it('compares two empty mediums', () => {
			const emptyMedium = undefined;
			const anotherEmptyMedium = undefined;

			const comparison = Medium.haveEqualMetadata(emptyMedium, anotherEmptyMedium);

			expect(comparison).toBe(true);
		});

		it('compares an empty with a non-empty medium', () => {
			const emptyMedium = undefined;

			const emptyFirstComparison = Medium.haveEqualMetadata(emptyMedium, birdman);
			const emptySecondComparison = Medium.haveEqualMetadata(birdman, emptyMedium);

			expect(emptyFirstComparison).toBe(false);
			expect(emptySecondComparison).toBe(false);
			expect(emptyFirstComparison).toBe(emptySecondComparison);
		});

		it('compares two different non-empty mediums', () => {
			const wargames = new Medium('WarGames', 114 * 60 * 1000);

			const birdmanWargamesComparison = Medium.haveEqualMetadata(birdman, wargames);
			const wargamesBirdmanComparison = Medium.haveEqualMetadata(wargames, birdman);

			expect(birdmanWargamesComparison).toBe(false);
			expect(wargamesBirdmanComparison).toBe(false);
			expect(birdmanWargamesComparison).toBe(wargamesBirdmanComparison);
		});

		it('compares identical non-empty mediums', () => {
			const comparison = Medium.haveEqualMetadata(birdman, birdman);

			expect(comparison).toBe(true);
		});

		it('compares only metadata', () => {
			const playingBirdman = new Medium('Birdman', 116 * 60 * 1000, true, new PlayingPlaybackState(1337));
			const pausedBirdman = new Medium('Birdman', 116 * 60 * 1000, false, new PausedPlaybackState(42));

			const comparison = Medium.haveEqualMetadata(playingBirdman, pausedBirdman);

			expect(comparison).toBe(true);
		});

		it('compares name and length', () => {
			const bidman = new Medium('Bidman', 116 * 60 * 1000);
			const birdmanTrailer = new Medium('Birdman', 2 * 60 * 1000);

			const differentLengthComparison = Medium.haveEqualMetadata(birdman, birdmanTrailer);
			const differentNameComparison = Medium.haveEqualMetadata(birdman, bidman);

			expect(differentLengthComparison).toBe(false);
			expect(differentNameComparison).toBe(false);
		});
	});
});
