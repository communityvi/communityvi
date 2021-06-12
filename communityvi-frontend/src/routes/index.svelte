<script context="module" lang="ts">
	export const prerender = true;
</script>

<script lang="ts">
	import Hello from '$lib/components/Hello.svelte';
	import Registration from '$lib/components/Registration.svelte';
	import {WebSocketTransport} from '$lib/client/transport';
	import {Client} from '$lib/client/client';
	import Notifications from '$lib/components/Notifications.svelte';

	const transport = new WebSocketTransport(new URL('ws://localhost:8000/ws'));
	const client = new Client(transport);
</script>

<svelte:head>
	<title>Home</title>
</svelte:head>

<section>
	<Notifications />
	<Hello name="User" />
	<Registration {client} />
</section>
