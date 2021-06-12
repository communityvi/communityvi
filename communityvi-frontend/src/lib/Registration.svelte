<script lang="ts">
	import type {Client} from '$lib/client/client';
	import {registeredClient} from '$lib/stores';
	import {onDestroy} from 'svelte';

	export let client: Client;

	let registeredName: string;

	$: isRegistered = $registeredClient !== undefined;

	const unsubscribe = registeredClient.subscribe(registeredClient => {
		registeredName = registeredClient?.name ?? '';
	});

	async function submit() {
		if (isRegistered) {
			$registeredClient?.logout();
		} else {
			$registeredClient = await client.register(registeredName.trim(), () => {
				console.info('Client disconnected.');
				$registeredClient = undefined;
			});
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
