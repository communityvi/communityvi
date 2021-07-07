<script lang="ts">
	import {registeredClient, notifications, videoUrl} from '$lib/stores';
	import type {Medium} from '$lib/client/model';
	import {onDestroy} from 'svelte';
	import type {MediumChangedByPeer, MediumTimeAdjusted} from '$lib/client/model';

	$: isRegistered = $registeredClient !== undefined;

	let medium: Medium | undefined;
	$: medium = $registeredClient?.currentMedium;
	$: mediumTitle = selectedMediumName ?? medium?.name ?? 'n/a';

	let formattedMediumLength: string;
	$: {
		const mediumLengthInMilliseconds = selectedMediumLengthInMilliseconds ?? medium?.lengthInMilliseconds;
		if (mediumLengthInMilliseconds !== undefined) {
			const lengthInMinutes = mediumLengthInMilliseconds / 1000 / 60;
			formattedMediumLength = `${Math.round(lengthInMinutes)} min.`;
		} else {
			formattedMediumLength = 'n/a';
		}
	}

	$: unsubscribe = $registeredClient?.subscribeToMediumStateChanges(onMediumStateChanged);

	let selectedMediumName: string | undefined;
	let selectedMediumLengthInMilliseconds: number | undefined;
	let selectedMediumUrl: string | undefined;

	let durationHelper: HTMLVideoElement;

	onDestroy(() => {
		if (unsubscribe !== undefined) {
			unsubscribe();
		}
	});

	function onMediumStateChanged(change: MediumChangedByPeer | MediumTimeAdjusted): void {
		resetMediumSelection();
		medium = change.medium;
	}

	function onMediumSelection(event: Event) {
		const element = event.target as HTMLInputElement;
		const medium = element?.files?.item(0) ?? undefined;
		if (medium === undefined) {
			return;
		}

		selectedMediumName = medium.name;
		selectedMediumUrl = URL.createObjectURL(medium);

		durationHelper.src = selectedMediumUrl;
		durationHelper.load();
	}

	async function onDurationHelperLoadedMetadata() {
		if ($registeredClient === undefined || selectedMediumName === undefined) {
			return;
		}

		selectedMediumLengthInMilliseconds = durationHelper.duration * 1000;
		$videoUrl = durationHelper.src;

		try {
			await $registeredClient.insertFixedLengthMedium(selectedMediumName, selectedMediumLengthInMilliseconds);
			medium = $registeredClient.currentMedium;
		} catch (error) {
			console.error('Error while inserting medium:', error);
			notifications.reportError(new Error(`Inserting new medium name '${selectedMediumName}' failed!`));
			resetMediumSelection();
		}
	}

	async function ejectMedium() {
		if ($registeredClient === undefined) {
			return;
		}

		resetMediumSelection();
		try {
			await $registeredClient.ejectMedium();
			medium = undefined;
		} catch (error) {
			console.error('Error while ejecting medium:', error);
			notifications.reportError(new Error('Ejecting the medium failed!'));
		}
	}

	function resetMediumSelection() {
		selectedMediumName = undefined;
		selectedMediumLengthInMilliseconds = undefined;
		selectedMediumUrl = undefined;
	}
</script>

{#if isRegistered}
	<section id="medium-selection">
		<span class="medium-title">{mediumTitle}</span>
		<span class="medium-duration">&nbsp;({formattedMediumLength})</span>

		<div class="file">
			<label class="file-label">
				<input class="file-input" type="file" accept="video/*,audio/*" on:change={onMediumSelection} />
				<span class="file-cta">
					<span class="file-icon">
						<i class="fas fa-upload" />
					</span>
					<span class="file-label">Insert Medium… </span>
				</span>
			</label>
		</div>
		<video
			preload="metadata"
			hidden={true}
			muted={true}
			bind:this={durationHelper}
			on:loadedmetadata={onDurationHelperLoadedMetadata}
		/>

		{#if medium !== undefined}
			<button on:click={ejectMedium}>Eject Medium</button>
		{/if}
	</section>
{/if}

<style lang="sass">
	.medium-title
		font-weight: bold
		font-size: 1.5em
</style>
