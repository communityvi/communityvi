<script context="module" lang="ts">
	export const prerender = true;
</script>

<script lang="ts">
	import Fa from 'svelte-fa/src/fa.svelte';
	import {faHouseUser} from '@fortawesome/free-solid-svg-icons';
	import Hello from '$lib/Hello.svelte';
	import Registration from '$lib/Registration.svelte';
	import {WebSocketTransport} from '../lib/client/transport';
	import {Client} from '$lib/client/client';

	const transport = new WebSocketTransport('ws://localhost:8000/ws');
	const client = new Client(transport);
</script>

<svelte:head>
	<title>Home</title>
</svelte:head>

<section>
	<Fa icon={faHouseUser} />
	<Hello name="User" />
	<Registration {client} />
</section>
