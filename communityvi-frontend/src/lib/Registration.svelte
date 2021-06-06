<script lang="ts">
	import type {Client} from '$lib/client/client';
	import {registeredClientStore} from '$lib/stores';
	import {onDestroy} from 'svelte';

	export let client: Client;

	let registeredName: string;
	let isRegistered = false;

	const unsubscribe = registeredClientStore.subscribe(registeredClient => {
		registeredName = registeredClient?.name ?? '';
		isRegistered = registeredClient !== undefined;
	});

	async function submit() {
		if (isRegistered) {
			registeredClientStore.update(registeredClient => {
				registeredClient?.logout();

				// FIXME: This should really happen in the `ClosedCallback`.
				return undefined;
			});
		} else {
			const registeredClient = await client.register(registeredName.trim());
			registeredClientStore.set(registeredClient);
		}
	}

	onDestroy(unsubscribe);
</script>

<form on:submit|preventDefault={submit}>
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
				<input class="button is-primary" type="submit" value="Login" disabled={registeredName === '' || undefined} />
			{/if}
		</p>
	</div>
</form>
