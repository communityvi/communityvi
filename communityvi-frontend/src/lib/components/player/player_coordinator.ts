import {PausedPlaybackState, PlayingPlaybackState} from '$lib/client/model';
import RateLimiter from '$lib/utils/rate_limiter';

export default class PlayerCoordinator {
	private readonly player: HTMLMediaElement;

	private lastPlaybackState: PlayingPlaybackState | PausedPlaybackState;

	// If true, the next seek was most likely caused by us, not the user, so ignore it.
	private ignoreNextSeek = false;

	private readonly rateLimiter = new RateLimiter(1000);

	private readonly playCallback: PlayCallback;
	private readonly pauseCallback: PauseCallback;

	static async forPlayerWithInitialState(
		player: HTMLMediaElement | null | undefined,
		initialPlaybackState: PlayingPlaybackState | PausedPlaybackState | undefined,
		playCallback: PlayCallback,
		pauseCallback: PauseCallback,
	): Promise<PlayerCoordinator | undefined> {
		if (!player || initialPlaybackState === undefined) {
			return undefined;
		}

		const playerStateManager = new PlayerCoordinator(player, initialPlaybackState, playCallback, pauseCallback);

		// Ensure the playback position is synchronized after reconnect
		await playerStateManager.syncPlaybackPosition(initialPlaybackState);

		return playerStateManager;
	}

	private constructor(
		player: HTMLMediaElement,
		initialPlaybackState: PlayingPlaybackState | PausedPlaybackState,
		playCallback: PlayCallback,
		pauseCallback: PauseCallback,
	) {
		this.player = player;

		this.player.onloadeddata = () => this.onLoadedData();
		this.player.onplay = () => this.onPlay();
		this.player.onpause = () => this.onPause();
		this.player.onseeked = () => this.onSeeked();

		this.lastPlaybackState = initialPlaybackState;

		this.playCallback = playCallback;
		this.pauseCallback = pauseCallback;
	}

	async setPlaybackState(playbackState?: PlayingPlaybackState | PausedPlaybackState): Promise<void> {
		if (playbackState === undefined) {
			return;
		}

		this.lastPlaybackState = playbackState;
		await this.syncPlaybackPosition(playbackState);
	}

	async resetPlaybackState(): Promise<void> {
		await this.syncPlaybackPosition(this.lastPlaybackState);
	}

	private async onLoadedData(): Promise<void> {
		await this.syncPlaybackPosition(this.lastPlaybackState);
	}

	private async syncPlaybackPosition(playbackState: PlayingPlaybackState | PausedPlaybackState): Promise<void> {
		// Resetting the rate limiter is really important so that local operations can never be
		// reordered to happen after an operation broadcast from the server.
		this.rateLimiter.reset();

		if (playbackState instanceof PlayingPlaybackState) {
			const proposedCurrentTime = performance.now() - playbackState.localStartTimeInMilliseconds;
			if (Math.abs(this.player.currentTime - (proposedCurrentTime / 1000)) > 1) {
				this.setPlayerPosition(proposedCurrentTime);
			}
			if (this.player.paused) {
				await this.player.play();
			}

			return;
		}

		this.setPlayerPosition(playbackState.positionInMilliseconds);
		if (!this.player.paused) {
			this.player.pause();
		}
	}

	private setPlayerPosition(milliseconds: number) {
		// Assigning to player.currentTime always seems to trigger a 'seeked' event
		// so we need to ignore the next one in order to differentiate it from
		// user triggered seeks.
		this.ignoreNextSeek = true;
		this.player.currentTime = milliseconds / 1000;
	}

	private onPlay(): void {
		if (this.player.seeking) {
			// If this event was triggered by seeking, ignore it because it is not an actual user
			// triggered event.
			return;
		}

		this.notifyPlayCallback(false);
	}

	private onPause(): void {
		if (this.player.seeking) {
			return;
		}

		this.notifyPauseCallback(false);
	}

	private onSeeked(): void {
		if (this.ignoreNextSeek) {
			this.ignoreNextSeek = false;
			return;
		}

		if (this.player.paused) {
			this.notifyPauseCallback(true);
		} else {
			this.notifyPlayCallback(true);
		}
	}

	private notifyPlayCallback(skipped: boolean): void {
		const startTimeInMilliseconds = performance.now() - this.player.currentTime * 1000;
		this.rateLimiter.call(() => this.playCallback(startTimeInMilliseconds, skipped));
	}

	private notifyPauseCallback(skipped: boolean): void {
		const positionInMilliseconds = this.player.currentTime * 1000;
		this.rateLimiter.call(() => this.pauseCallback(positionInMilliseconds, skipped));
	}
}

type PlayCallback = (startTimeInMilliseconds: number, skipped: boolean) => void;
type PauseCallback = (positionInMilliseconds: number, skipped: boolean) => void;
