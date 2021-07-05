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
	// REALLY BAD!!!! hack to work around our inability to determine if an event has been triggered by
	// ourselves (because of assignments to currentTime or calling play/pause) or by the user.
	// This works by assigning 1 for every operation we do ourselves that we anticipate to trigger an event.
	let ignoreNextPlay = 0;
	let ignoreNextPause = 0;

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
			ignoreNextPlay = 1;
			if (player.paused) {
				ignoreNextPlay = 2;
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
				ignoreNextPause = 1;
				player.pause();
			}
		}
	}

	async function onPlaying() {
		console.log('ignoreNextPlay', ignoreNextPlay);
		if (ignoreNextPlay > 0) {
			ignoreNextPlay--;
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
		console.log('ignoreNextPause', ignoreNextPause);
		if (ignoreNextPause > 0) {
			ignoreNextPause--;
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
				ignoreNextPause = 1;
				player.pause();
			}
			return;
		}

		if (medium.playbackState instanceof PlayingPlaybackState) {
			player.currentTime = (performance.now() - lastStartTimeInMilliseconds) / 1000;
			ignoreNextPlay = 1;
			if (player.paused) {
				ignoreNextPlay = 2;
				await player.play();
			}
			return;
		}
	}

	function onSeeked(event: Event) {
		console.log('seeked', event);
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
			on:playing={onPlaying}
			on:seeked={onSeeked}
		/>
	</section>
{/if}
