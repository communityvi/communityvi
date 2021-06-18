<script context="module" lang="ts">
	export const prerender = true;
</script>

<script lang="ts">
	import Registration from '$lib/components/Registration.svelte';
	import {WebSocketTransport} from '$lib/client/transport';
	import {Client} from '$lib/client/client';

	const transport = new WebSocketTransport(new URL('ws://localhost:8000/ws'));
	const client = new Client(transport);
</script>

<svelte:head>
	<title>Home</title>
</svelte:head>

<section>
	<Registration {client} />
</section>
