<script lang="ts">
	import {registeredClient, notifications, videoUrl} from '$lib/stores';
	import {Medium, MediumChangedByOurself} from '$lib/client/model';
	import {onDestroy} from 'svelte';
	import type {MediumChangedByPeer, MediumTimeAdjusted} from '$lib/client/model';

	$: isRegistered = $registeredClient !== undefined;

	let medium: Medium | undefined;
	$: {
		if ($registeredClient !== undefined) {
			if (!Medium.haveEqualMetadata(medium, $registeredClient.currentMedium)) {
				// Update the medium in case of relogin
				medium = $registeredClient.currentMedium;
				selectedMediumName = medium?.name;
				selectedMediumLengthInMilliseconds = medium?.lengthInMilliseconds;
				$videoUrl = undefined;
			}
		}
	}
	$: mediumIsOutdated = medium !== undefined && $videoUrl === undefined;

	let formattedMediumLength: string | undefined;
	$: {
		const mediumLengthInMilliseconds = medium?.lengthInMilliseconds;
		formattedMediumLength = undefined;
		if (mediumLengthInMilliseconds !== undefined) {
			const lengthInMinutes = mediumLengthInMilliseconds / 1000 / 60;
			formattedMediumLength = `${Math.round(lengthInMinutes)} min.`;
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

	function onMediumStateChanged(change: MediumChangedByPeer | MediumChangedByOurself | MediumTimeAdjusted): void {
		if (change instanceof MediumChangedByOurself) {
			return;
		}

		resetMediumSelection();
		if (!Medium.haveEqualMetadata(medium, change.medium)) {
			$videoUrl = undefined;
		}
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

		selectedMediumLengthInMilliseconds = Math.round(durationHelper.duration * 1000);
		if (
			mediumIsOutdated &&
			medium !== undefined &&
			(selectedMediumName !== medium.name || selectedMediumLengthInMilliseconds != medium.lengthInMilliseconds)
		) {
			notifications.error('Wrong medium selected');
			return;
		}

		$videoUrl = durationHelper.src;

		try {
			if (mediumIsOutdated) {
				return;
			}
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
			$videoUrl = undefined;
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
		<span class="medium-title">{medium?.name ?? 'n/a'}</span>
		<span class="medium-duration">&nbsp;({formattedMediumLength ?? 'n/a'})</span>

		<div class="file">
			<label class="file-label">
				<input class="file-input" type="file" accept="video/*,audio/*" on:change={onMediumSelection} />
				<span class="file-cta">
					<span class="file-icon">
						<i class="fas fa-upload" />
					</span>
					<span class="file-label">
						{#if mediumIsOutdated && medium?.name !== undefined}
							Select file for "{medium.name}"
						{:else}
							Insert New Medium…
						{/if}
					</span>
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
