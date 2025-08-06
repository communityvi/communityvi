<script lang="ts">
	import {notifications, registeredClient, videoUrl} from '$lib/stores';
	import {onDestroy} from 'svelte';
	import PlayerCoordinator from '$lib/components/player/player_coordinator';
	import type {Medium} from '$lib/client/model';

	// NOTE: currentTime needs to be accessed via the player itself because the binding provided by svelte
	// as of 3.38.3 neither reads the currentTime reliably, nor sets it reliably.
	let player: HTMLVideoElement | undefined = $state(undefined);

	// NOTE: Reactively act on registeredClient (e.g. reconnect) and videoUrl (e.g. local medium selection)
	// changed to catch all cases in which the player or its position require updating.
	// Important: $registeredClient and $videoUrl are explicitly mentioned to trigger the reactive updates.
	let playerCoordinator: PlayerCoordinator | undefined;
	$effect(() => {
		if ($registeredClient || $videoUrl) {
			initializeOrUpdatePlayerState();
		}
	});
	// NOTE: Can't use $derived because we need the side-effect of subscribing to state changes
	let unsubscribe: (() => void) = $state(() => {});
	$effect(() => {
		if ($registeredClient === undefined) {
			unsubscribe = () => {};
			return;
		}

		unsubscribe = $registeredClient.subscribeToMediumStateChanges(async change => {
			await initializeOrUpdatePlayerState(change.medium);
		});
	});

	onDestroy(() => {
		unsubscribe();
	});

	async function initializeOrUpdatePlayerState(medium?: Medium) {
		if (playerCoordinator !== undefined) {
			const currentPlaybackState = medium?.playbackState ?? $registeredClient?.currentMedium?.playbackState;
			await playerCoordinator.setPlaybackState(currentPlaybackState);
			return;
		}

		const initialPlaybackState = $registeredClient?.currentMedium?.playbackState;
		playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
			player,
			initialPlaybackState,
			onPlay,
			onPause,
		);
	}

	async function onPlay(startTimeInMilliseconds: number, skipped: boolean) {
		try {
			await $registeredClient?.play(startTimeInMilliseconds, skipped);
		} catch (error) {
			await handleStateChangeError(error as Error, skipped, getPlayStateChangedErrorMessage);
		}
	}

	function getPlayStateChangedErrorMessage(_error: Error, skipped: boolean) {
		if (skipped) {
			return 'Skipping during playback failed! Resetting playback state.';
		}

		return 'Server rejected the play state! Resetting playback state.';
	}

	async function onPause(positionInMilliseconds: number, skipped: boolean) {
		try {
			await $registeredClient?.pause(positionInMilliseconds, skipped);
		} catch (error) {
			await handleStateChangeError(error as Error, skipped, getPauseStateChangedErrorMessage);
		}
	}

	function getPauseStateChangedErrorMessage(_error: Error, skipped: boolean) {
		if (skipped) {
			return 'Skipping during pause failed! Resetting playback state.';
		}

		return 'Server rejected the pause state! Resetting playback state.';
	}

	async function handleStateChangeError(
		error: Error,
		skipped: boolean,
		messageBuilder: (error: Error, skipped: boolean) => string,
	) {
		notifications.inform(messageBuilder(error, skipped));
		notifications.reportError(error);
		await playerCoordinator?.resetPlaybackState();
	}
</script>

<!-- svelte-ignore a11y_media_has_caption -->
<section id="player" class:is-hidden={$videoUrl === undefined}>
	<video controls src={$videoUrl ?? ''} bind:this={player}></video>
</section>
