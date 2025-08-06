import {PausedPlaybackState, PlayingPlaybackState} from '$lib/client/model';
import PlayerCoordinator from '$lib/components/player/player_coordinator';
import TimeMock from '../client/helper/time_mock';
import {describe, it, expect, vi} from 'vitest';
import {mock} from 'vitest-mock-extended';

describe('The PlayerCoordinator', () => {
	describe('forPlayerWithInitialSate factory', () => {
		it('constructs a PlayerCoordinator', async () => {
			const player = mock<HTMLMediaElement>();
			const initialPlaybackState = new PausedPlaybackState(0);
			const callback = vi.fn();

			const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
				player,
				initialPlaybackState,
				callback,
				callback,
			);

			expect(playerCoordinator).toBeInstanceOf(PlayerCoordinator);
		});

		it('ignores undefined initial playback state', async () => {
			const player = mock<HTMLMediaElement>();
			const initialPlaybackState = undefined;
			const callback = vi.fn();

			const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
				player,
				initialPlaybackState,
				callback,
				callback,
			);

			expect(playerCoordinator).toBeUndefined();
		});

		it('syncs the player with the initial playback state', async () => {
			const player = mock<HTMLMediaElement>({
				currentTime: 42,
				paused: false,
			});
			const pausePosition = 1337;
			const initialPlaybackState = new PausedPlaybackState(pausePosition);
			const callback = vi.fn();

			await PlayerCoordinator.forPlayerWithInitialState(player, initialPlaybackState, callback, callback);

			expect(player.currentTime).toBe(pausePosition / 1000);
			expect(player.pause).toHaveBeenCalledTimes(1);
		});
	});

	describe('updating the player based on a new playback state from the server', () => {
		it('pauses a playing player', async () => {
			const player = mock<HTMLMediaElement>({
				paused: false,
			});
			const initialPlaybackState = new PlayingPlaybackState(0);
			const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
				player,
				initialPlaybackState,
				vi.fn(),
				vi.fn(),
			);

			playerCoordinator?.setPlaybackState(new PausedPlaybackState(42));

			expect(player.pause).toHaveBeenCalledTimes(1);
			expect(player.currentTime).toBe(0.042);
		});

		it('skips an already paused player', async () => {
			const initialPosition = 42;
			const player = mock<HTMLMediaElement>({
				paused: true,
				currentTime: initialPosition / 1000,
			});
			const initialPlaybackState = new PausedPlaybackState(initialPosition);
			const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
				player,
				initialPlaybackState,
				vi.fn(),
				vi.fn(),
			);

			playerCoordinator?.setPlaybackState(new PausedPlaybackState(1_337));

			expect(player.pause).not.toHaveBeenCalled();
			expect(player.play).not.toHaveBeenCalled();
			expect(player.currentTime).toBe(1.337);
		});

		it('starts a paused player', async () => {
			await TimeMock.run(async () => {
				const initialPerformanceNow = performance.now();

				const player = mock<HTMLMediaElement>({
					paused: true,
				});
				const initialPlaybackState = new PausedPlaybackState(0);
				const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
					player,
					initialPlaybackState,
					vi.fn(),
					vi.fn(),
				);

				playerCoordinator?.setPlaybackState(new PlayingPlaybackState(42));

				expect(player.play).toHaveBeenCalledTimes(1);
				expect(player.currentTime).toBe((initialPerformanceNow - 42) / 1000);
			});
		});

		it('skips an already playing player if new position is above threshold', async () => {
			await TimeMock.run(async () => {
				const initialPerformanceNow = performance.now();

				const player = mock<HTMLMediaElement>({
					paused: false,
					currentTime: 0,
				});
				const initialPlaybackState = new PlayingPlaybackState(initialPerformanceNow);
				const thresholdMilliseconds = 1000;
				const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
					player,
					initialPlaybackState,
					vi.fn(),
					vi.fn(),
					thresholdMilliseconds,
				);

				const localPositionAboveThreshold = initialPerformanceNow - thresholdMilliseconds;
				playerCoordinator?.setPlaybackState(new PlayingPlaybackState(localPositionAboveThreshold));

				expect(player.play).not.toHaveBeenCalled();
				expect(player.pause).not.toHaveBeenCalled();
				expect(player.currentTime).toBe(thresholdMilliseconds / 1000);
			});
		});

		it('does not skip an already playing player if the new position is below the threshold', async () => {
			await TimeMock.run(async () => {
				const initialPerformanceNow = performance.now();

				const player = mock<HTMLMediaElement>({
					paused: false,
					currentTime: 0,
				});
				const initialPlaybackState = new PlayingPlaybackState(initialPerformanceNow);
				const thresholdMilliseconds = 1000;
				const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
					player,
					initialPlaybackState,
					vi.fn(),
					vi.fn(),
					thresholdMilliseconds,
				);

				const localPositionBelowThreshold = initialPerformanceNow - (thresholdMilliseconds - 1);
				playerCoordinator?.setPlaybackState(new PlayingPlaybackState(localPositionBelowThreshold));

				expect(player.play).not.toHaveBeenCalled();
				expect(player.pause).not.toHaveBeenCalled();
				expect(player.currentTime).toBe(0);
			});
		});
	});

	// TODO: resetPlaybackState
	// TODO: constant seeking doesn't spam skips (rate limiter)
	// TODO: forwarding of playback changes caused by the user
	// TODO: differentiates between skips caused by the user and caused by the playercoordinator itself (needs an integration test)
});
