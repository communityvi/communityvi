<script lang="ts">
	import {registeredClient, videoUrl, notifications} from '$lib/stores';
	import {onDestroy} from 'svelte';
	import type {MediumChangedByPeer, MediumTimeAdjusted} from '$lib/client/model';
	import {PausedPlaybackState, PlayingPlaybackState} from '$lib/client/model';

	// NOTE: currentTime needs to be accessed via the player itself because the binding provided by svelte
	// as of 3.38.3 neither reads the currentTime reliably, nor sets it reliably.
	let player: HTMLVideoElement;
	let lastStartTimeInMilliseconds = 0;
	let lastPositionInMilliseconds = 0;

	$: unsubscribe = $registeredClient?.subscribeToMediumStateChanges(onMediumStateChanged);

	onDestroy(() => {
		if (unsubscribe !== undefined) {
			unsubscribe();
		}
	});

	async function onMediumStateChanged(change: MediumChangedByPeer | MediumTimeAdjusted): Promise<void> {
		const playbackState = change.medium?.playbackState;
		if (playbackState instanceof PlayingPlaybackState) {
			lastStartTimeInMilliseconds = playbackState.localStartTimeInMilliseconds;
			player.currentTime = (performance.now() - playbackState.localStartTimeInMilliseconds) / 1000;
			console.log('playing at current time:', player.currentTime);
			if (player.paused) {
				console.log('about to start player');
				await player.play();
			}
			return;
		}

		if (playbackState instanceof PausedPlaybackState) {
			lastPositionInMilliseconds = playbackState.positionInMilliseconds;
			player.currentTime = playbackState.positionInMilliseconds / 1000;
			console.log('paused at current time:', player.currentTime);
			if (!player.paused) {
				player.pause();
			}
		}
	}

	async function onPlay() {
		if (player.seeking) {
			// If this pause event was triggered by seeking, ignore it because it is not an actual user
			// triggered pause.
			return;
		}

		const localStartTimeInMilliseconds = performance.now() - player.currentTime * 1000;
		try {
			await $registeredClient?.play(localStartTimeInMilliseconds);
		} catch (error) {
			notifications.reportError(error);
			await resetPlaybackState();
		}
	}

	async function onPause() {
		if (player.seeking) {
			// If this pause event was triggered by seeking, ignore it because it is not an actual user
			// triggered pause.
			return;
		}

		try {
			await $registeredClient?.pause(player.currentTime * 1000);
		} catch (error) {
			notifications.reportError(error);
			await resetPlaybackState();
		}
	}

	async function resetPlaybackState() {
		notifications.inform('Resetting playback state.');

		const medium = $registeredClient?.currentMedium;
		if (medium === undefined) {
			return;
		}

		if (medium.playbackState instanceof PausedPlaybackState) {
			player.currentTime = lastPositionInMilliseconds / 1000;
			if (!player.paused) {
				player.pause();
			}
			return;
		}

		if (medium.playbackState instanceof PlayingPlaybackState) {
			player.currentTime = (performance.now() - lastStartTimeInMilliseconds) / 1000;
			if (player.paused) {
				await player.play();
			}
			return;
		}
	}
</script>

{#if $videoUrl !== undefined}
	<section id="player">
		<video
			width="640"
			height="360"
			controls
			src={$videoUrl}
			muted={true}
			bind:this={player}
			on:pause={onPause}
			on:play={onPlay}
		/>
	</section>
{/if}
