<script context="module" lang="ts">
	export const prerender = true;
</script>

<script lang="ts">
	import Registration from '$lib/components/Registration.svelte';
	import {WebSocketTransport} from '$lib/client/transport';
	import Client from '$lib/client/client';
	import Chat from '$lib/components/chat/Chat.svelte';
	import MediumSelector from '$lib/components/medium_selector/MediumSelector.svelte';
	import Peers from '$lib/components/Peers.svelte';
	import Player from '$lib/components/player/Player.svelte';
	import {page} from '$app/stores';

	const transport = new WebSocketTransport(determineBackendURL());
	const client = new Client(transport);

	function determineBackendURL(): URL {
		// Just a stopgap measure for now. The (generally wrong) assumption is that
		// the backend listens on port 8000. But this is good enough for now because it
		// works both with `npm run watch` and the default backend settings as well
		// as when the frontend is bundled with the backend.
		const url = new URL(`ws://${$page.host}/ws`);
		url.port = '8000';

		return url;
	}
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
