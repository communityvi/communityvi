<script lang="ts">
	import {notifications} from '$lib/stores';
	import {onDestroy} from 'svelte';
	import PlayerCoordinator from '$lib/components/player/player_coordinator';
	import type {Medium} from '$lib/client/model';
	import RegisteredClient from '$lib/client/registered_client';

	interface Properties {
		videoUrl: string;
		registeredClient?: RegisteredClient;
	}

	let {videoUrl, registeredClient}: Properties = $props();

	let player: HTMLVideoElement;

	// NOTE: Reactively act on registeredClient (e.g. reconnect) and videoUrl (e.g. local medium selection)
	// changed to catch all cases in which the player or its position require updating.
	// Important: $registeredClient is explicitly mentioned to trigger the reactive updates.
	let playerCoordinator: PlayerCoordinator | undefined;
	$effect(() => {
		if (registeredClient) {
			initializeOrUpdatePlayerState();
		}
	});
	// NOTE: Can't use $derived because we need the side-effect of subscribing to state changes
	let unsubscribe: (() => void) = $state(() => {});
	$effect(() => {
		if (registeredClient === undefined) {
			unsubscribe = () => {};
			return;
		}

		unsubscribe = registeredClient.subscribeToMediumStateChanges(async change => {
			await initializeOrUpdatePlayerState(change.medium);
		});
	});

	onDestroy(() => {
		unsubscribe();
	});

	async function initializeOrUpdatePlayerState(medium?: Medium) {
		if (playerCoordinator !== undefined) {
			const currentPlaybackState = medium?.playbackState ?? registeredClient?.currentMedium?.playbackState;
			await playerCoordinator.setPlaybackState(currentPlaybackState);
			return;
		}

		const initialPlaybackState = registeredClient?.currentMedium?.playbackState;
		playerCoordinator = await PlayerCoordinator.forPlayerWithInitialState(
			player,
			initialPlaybackState,
			onPlay,
			onPause,
		);
	}

	async function onPlay(startTimeInMilliseconds: number, skipped: boolean) {
		try {
			await registeredClient?.play(startTimeInMilliseconds, skipped);
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
			await registeredClient?.pause(positionInMilliseconds, skipped);
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
<section id="player">
	<video controls src={videoUrl} bind:this={player}></video>
</section>
