<script lang="ts">
	import {registeredClient, videoUrl} from '$lib/stores';
	import {onDestroy} from 'svelte';
	import type {MediumChangedByPeer, MediumTimeAdjusted} from '$lib/client/model';
	import {PausedPlaybackState, PlayingPlaybackState} from '$lib/client/model';

	let currentTime = 0;
	let paused: boolean;

	$: unsubscribe = $registeredClient?.subscribeToMediumStateChanges(onMediumStateChanged);

	onDestroy(() => {
		if (unsubscribe !== undefined) {
			unsubscribe();
		}
	});

	function onMediumStateChanged(change: MediumChangedByPeer | MediumTimeAdjusted): void {
		const playbackState = change.medium?.playbackState;
		if (playbackState instanceof PlayingPlaybackState) {
			paused = false;
			currentTime = (performance.now() - playbackState.localStartTimeInMilliseconds) / 1000;
			console.log('set current time:', currentTime);
			return;
		}

		if (playbackState instanceof PausedPlaybackState) {
			paused = true;
			currentTime = playbackState.positionInMilliseconds / 1000;
		}
	}
</script>

{#if $videoUrl !== undefined}
	<section id="player">
		<video width="640" height="360" controls src={$videoUrl} muted={true} bind:currentTime bind:paused />
	</section>
{/if}
