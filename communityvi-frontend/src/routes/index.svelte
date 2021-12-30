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
	import {browser} from '$app/env';
	import {RESTClient} from '$lib/client/RESTClient';

	const transport = new WebSocketTransport(determineWebSocketURL());
	const restClient = new RESTClient(determineAPIUrl());
	const client = new Client(transport, restClient);

	function determineWebSocketURL(): URL {
		// Just a stopgap measure for now. The (generally wrong) assumption is that
		// the backend listens on port 8000. But this is good enough for now because it
		// works both with `npm run watch` and the default backend settings as well
		// as when the frontend is bundled with the backend.
		const url = new URL(`ws://${pageHost()}/ws`);
		url.port = '8000';

		return url;
	}

	function determineAPIUrl(): URL {
		// Just a stopgap measure for now. The (generally wrong) assumption is that
		// the backend listens on port 8000. But this is good enough for now because it
		// works both with `npm run watch` and the default backend settings as well
		// as when the frontend is bundled with the backend.
		const url = new URL(`http://${pageHost()}/api`);
		url.port = '8000';

		return url;
	}

	function pageHost(): string {
		if (browser) {
			return new URL(window.location.href).host;
		} else {
			return $page.url.host;
		}
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
