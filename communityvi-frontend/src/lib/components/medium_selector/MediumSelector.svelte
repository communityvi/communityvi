<script lang="ts">
	import {registeredClient, notifications, videoUrl} from '$lib/stores';
	import {Medium, MediumChangedByOurself} from '$lib/client/model';
	import type {MediumStateChanged} from '$lib/client/model';
	import {onDestroy} from 'svelte';
	import {formatMediumLength} from '$lib/components/medium_selector/helpers';
	import MetadataLoader, {SelectedMedium} from '$lib/components/medium_selector/metadata_loader';

	let isRegistered = $derived($registeredClient !== undefined);

	let medium: Medium | undefined = $state();
	$effect(() => {
		if ($registeredClient !== undefined && Medium.hasChangedMetadata(medium, $registeredClient.currentMedium)) {
			// Update the medium in case of relogin
			medium = $registeredClient.currentMedium;
			$videoUrl = undefined;
		}
	});
	let mediumIsOutdated = $derived(medium !== undefined && $videoUrl === undefined);

	let durationHelper: HTMLVideoElement | undefined = $state();
	let metadataLoader = $derived(durationHelper ? new MetadataLoader(durationHelper) : undefined);

	let fileSelector: HTMLInputElement = $state();

	let unsubscribe: (() => void) | undefined = $state(undefined);
	$effect(() => {
		unsubscribe = $registeredClient?.subscribeToMediumStateChanges(onMediumStateChanged);
	});

	onDestroy(() => {
		if (unsubscribe !== undefined) {
			unsubscribe();
		}
	});

	function onMediumStateChanged(change: MediumStateChanged): void {
		if (change instanceof MediumChangedByOurself) {
			return;
		}

		if (Medium.hasChangedMetadata(medium, change.medium)) {
			$videoUrl = undefined;
		}

		medium = change.medium;
	}

	async function onMediumSelection() {
		const selectedFile = fileSelector.files?.item(0) ?? undefined;
		if (selectedFile === undefined || metadataLoader === undefined) {
			return;
		}

		let selectedMedium: SelectedMedium;
		try {
			selectedMedium = await metadataLoader.selectedMediumFromFile(selectedFile);
		} catch (error) {
			console.error('Error while loading medium:', error);
			notifications.reportError(error as Error);
			return;
		}

		if (mediumIsOutdated && medium !== undefined && selectedMedium.isMeaningfullyDifferentTo(medium)) {
			notifications.error('Wrong medium selected');
			return;
		}

		$videoUrl = URL.createObjectURL(selectedFile);

		if (mediumIsOutdated || $registeredClient === undefined) {
			return;
		}

		try {
			await $registeredClient.insertFixedLengthMedium(selectedMedium.name, selectedMedium.lengthInMilliseconds);
			medium = $registeredClient.currentMedium;
		} catch (error) {
			console.error('Error while inserting medium:', error);
			notifications.reportError(new Error(`Inserting new medium name '${selectedMedium.name}' failed!`));
		}
	}

	async function ejectMedium() {
		// The input element needs to be reset, otherwise Chrome won't trigger a change event
		// if the same file is selected again after ejecting.
		// See https://github.com/communityvi/communityvi/issues/267
		resetFileSelector();

		if ($registeredClient === undefined) {
			return;
		}

		try {
			await $registeredClient.ejectMedium();
			medium = undefined;
			$videoUrl = undefined;
		} catch (error) {
			console.error('Error while ejecting medium:', error);
			notifications.reportError(new Error('Ejecting the medium failed!'));
		}
	}

	function resetFileSelector() {
		fileSelector.value = '';
	}
</script>

<!-- Hidden video element for parsing file metadata -->
<video hidden={true} muted={true} bind:this={durationHelper}></video>

{#if isRegistered}
	<section id="medium-selection">
		<span class="medium-title">{medium?.name ?? 'n/a'}</span>
		<span class="medium-duration">&nbsp;({medium ? formatMediumLength(medium) : 'n/a'})</span>

		<div class="file">
			<label class="file-label">
				<input
					class="file-input"
					type="file"
					accept="video/*,audio/*"
					bind:this={fileSelector}
					onchange={onMediumSelection}
				/>
				<span class="file-cta">
					<span class="file-icon">
						<i class="fas fa-upload"></i>
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

		{#if medium !== undefined}
			<button onclick={ejectMedium}>Eject Medium</button>
		{/if}
	</section>
{/if}

<style lang="sass">
	.medium-title
		font-weight: bold
		font-size: 1.5em
</style>
