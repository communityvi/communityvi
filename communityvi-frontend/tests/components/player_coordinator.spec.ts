import {PausedPlaybackState} from '$lib/client/model';
import PlayerCoordinator from '$lib/components/player/player_coordinator';
import {mock} from 'jest-mock-extended';

describe('The PlayerCoordinator', () => {
	describe('forPlayerWithInitialSate factory', () => {
		it('constructs a PlayerCoordinator', async () => {
			const player = mock<HTMLMediaElement>();
			const initialPlaybackState = new PausedPlaybackState(0);
			const callback = jest.fn();

			const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
				player,
				initialPlaybackState,
				callback,
				callback,
			);

			expect(playerCoordinator).toBeInstanceOf(PlayerCoordinator);
		});

		it('ignores undefined player', async () => {
			const player = undefined;
			const initialPlaybackState = new PausedPlaybackState(0);
			const callback = jest.fn();

			const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
				player,
				initialPlaybackState,
				callback,
				callback,
			);

			expect(playerCoordinator).toBeUndefined();
		});

		it('ignores null player', async () => {
			const player = null;
			const initialPlaybackState = new PausedPlaybackState(0);
			const callback = jest.fn();

			const playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
				player,
				initialPlaybackState,
				callback,
				callback,
			);

			expect(playerCoordinator).toBeUndefined();
		});

		it('ignores undefined initial playback state', async () => {
			const player = mock<HTMLMediaElement>();
			const initialPlaybackState = undefined;
			const callback = jest.fn();

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
			const callback = jest.fn();

			await PlayerCoordinator.forPlayerWithInitialState(player, initialPlaybackState, callback, callback);

			expect(player.currentTime).toBe(pausePosition / 1000);
			expect(player.pause).toHaveBeenCalledTimes(1);
		});
	});
});

export {};
