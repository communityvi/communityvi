<script lang="ts">
	import {registeredClient, notifications, videoUrl} from '$lib/stores';
	import {Medium, MediumChangedByOurself} from '$lib/client/model';
	import {onDestroy} from 'svelte';
	import type {MediumChangedByPeer, MediumTimeAdjusted} from '$lib/client/model';
	import MetadataLoader from '$lib/components/metadata_loader';

	$: isRegistered = $registeredClient !== undefined;

	let medium: Medium | undefined;
	$: {
		if ($registeredClient !== undefined) {
			if (!Medium.haveEqualMetadata(medium, $registeredClient.currentMedium)) {
				// Update the medium in case of relogin
				medium = $registeredClient.currentMedium;
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

	let durationHelper: HTMLVideoElement | undefined;
	$: metadataLoader = durationHelper ? new MetadataLoader(durationHelper) : undefined;

	onDestroy(() => {
		if (unsubscribe !== undefined) {
			unsubscribe();
		}
	});

	function onMediumStateChanged(change: MediumChangedByPeer | MediumChangedByOurself | MediumTimeAdjusted): void {
		if (change instanceof MediumChangedByOurself) {
			return;
		}

		if (!Medium.haveEqualMetadata(medium, change.medium)) {
			$videoUrl = undefined;
		}
		medium = change.medium;
	}

	async function onMediumSelection(event: Event) {
		const fileSelector = event.target as HTMLInputElement;
		const selectedFile = fileSelector?.files?.item(0) ?? undefined;
		if (selectedFile === undefined || metadataLoader === undefined) {
			return;
		}

		let selectedMedium: Medium;
		try {
			selectedMedium = await metadataLoader.mediumFromFile(selectedFile);
		} catch (error) {
			console.error('Error while loading medium:', error);
			notifications.reportError(error);
			return;
		}

		if (mediumIsOutdated && medium !== undefined && !Medium.haveEqualMetadata(medium, selectedMedium)) {
			notifications.error('Wrong medium selected');
			return;
		}

		$videoUrl = URL.createObjectURL(selectedFile);

		try {
			if (mediumIsOutdated) {
				return;
			}
			await $registeredClient?.insertFixedLengthMedium(selectedMedium.name, selectedMedium.lengthInMilliseconds);
			medium = $registeredClient?.currentMedium;
		} catch (error) {
			console.error('Error while inserting medium:', error);
			notifications.reportError(new Error(`Inserting new medium name '${selectedMedium.name}' failed!`));
		}
	}

	async function ejectMedium() {
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
</script>

<!-- Hidden video element for parsing file metadata -->
<video hidden={true} muted={true} bind:this={durationHelper} />

{#if isRegistered}
	<section id="medium-selection">
		<span class="medium-title">{medium?.name ?? 'n/a'}</span>
		<span class="medium-duration">&nbsp;({formattedMediumLength ?? 'n/a'})</span>

		<div class="file">
			<label class="file-label">
				<!-- FIXME: The player seems to behave differently when working with audio :-( -->
				<input class="file-input" type="file" accept="video/*" on:change={onMediumSelection} />
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
