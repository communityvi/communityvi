<script lang="ts">
	import type Client from '$lib/client/client';
	import {notifications, registeredClient} from '$lib/stores';
	import {CloseReason} from '$lib/client/connection';

	interface Properties {
		client: Client;
	}

	let { client }: Properties = $props();

	// NOTE: Disabling ESLint here because we haven't found a better way to ensure that the registeredName
	//       is both bound to the text field and also updated when the registeredClient changes
	// eslint-disable-next-line svelte/prefer-writable-derived
	let registeredName = $state('');
	$effect(() => {
		registeredName = $registeredClient?.name ?? '';
	});
	let isRegistered = $derived($registeredClient !== undefined);

	async function submit() {
		if (isRegistered) {
			$registeredClient?.logout();
			return;
		}

		try {
			$registeredClient = await client.register(registeredName.trim(), onClose);
		} catch (error) {
			notifications.reportError(error as Error);
		}
	}

	function onClose(reason: CloseReason) {
		switch (reason) {
			case CloseReason.CLIENT_LEFT:
				console.info('Client disconnected.');
				break;
			case CloseReason.KICKED_FROM_SERVER:
				console.warn('User was kicked from server.');
				break;
			case CloseReason.ERROR:
				console.warn('The connection was closed due to a connection error.');
				break;
			default:
				console.error('Unknown close reason:', reason);
				break;
		}

		$registeredClient = undefined;
	}
</script>

<form onsubmit={submit}>
	<div class="field has-addons">
		<p class="control">
			<span class="button is-static">Username:</span>
		</p>

		<p class="control is-expanded">
			<input
				class="input"
				type="text"
				required
				min="1"
				bind:value={registeredName}
				disabled={isRegistered || undefined}
			/>
		</p>

		<p class="control">
			{#if isRegistered}
				<input class="button is-danger" type="submit" value="Logout" />
			{:else}
				<input
					class="button is-primary"
					type="submit"
					value="Login"
					disabled={registeredName === '' || undefined}
				/>
			{/if}
		</p>
	</div>
</form>
