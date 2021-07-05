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

	// If true, the next seek was most likely caused by us, not the user, so ignore it.
	let anticipatingSeek = false;

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
			setPlayerPosition(performance.now() - playbackState.localStartTimeInMilliseconds);
			if (player.paused) {
				await player.play();
			}
			return;
		}

		if (playbackState instanceof PausedPlaybackState) {
			lastPositionInMilliseconds = playbackState.positionInMilliseconds;
			setPlayerPosition(playbackState.positionInMilliseconds);
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

	async function onSeeked() {
		if (anticipatingSeek) {
			anticipatingSeek = false;
			return;
		}

		try {
			if (player.paused) {
				await $registeredClient?.pause(player.currentTime * 1000, true);
				return;
			}

			const localStartTimeInMilliseconds = performance.now() - player.currentTime * 1000;
			await $registeredClient?.play(localStartTimeInMilliseconds, true);
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
			setPlayerPosition(lastPositionInMilliseconds);

			if (!player.paused) {
				player.pause();
			}
			return;
		}

		if (medium.playbackState instanceof PlayingPlaybackState) {
			setPlayerPosition(performance.now() - lastStartTimeInMilliseconds);
			if (player.paused) {
				await player.play();
			}
			return;
		}
	}

	function setPlayerPosition(milliseconds: number) {
		// Assigning to player.currentTime always seems to trigger a seeked event
		// So we need to ignore the next one in order to differentiate it from
		// user triggered seeks.
		anticipatingSeek = true;
		player.currentTime = milliseconds / 1000;
	}
</script>

{#if $videoUrl !== undefined}
	<!-- svelte-ignore a11y-media-has-caption -->
	<section id="player">
		<video controls src={$videoUrl} bind:this={player} on:pause={onPause} on:play={onPlay} on:seeked={onSeeked} />
	</section>
{/if}
