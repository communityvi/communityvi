<script context="module" lang="ts">
	export const prerender = true;
</script>

<script lang="ts">
	import Registration from '$lib/components/Registration.svelte';
	import {WebSocketTransport} from '$lib/client/transport';
	import {Client} from '$lib/client/client';
	import Chat from '$lib/components/chat/Chat.svelte';
	import MediumSelector from '$lib/components/medium_selector/MediumSelector.svelte';
	import Peers from '$lib/components/Peers.svelte';
	import Player from '$lib/components/player/Player.svelte';

	const transport = new WebSocketTransport(new URL('ws://localhost:8000/ws'));
	const client = new Client(transport);
</script>

<svelte:head>
	<title>Home</title>
</svelte:head>

<section id="registration">
	<Registration {client} />
</section>

<MediumSelector />

<Player />

<Peers />

<Chat />
