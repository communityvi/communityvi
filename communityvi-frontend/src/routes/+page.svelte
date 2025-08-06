<script lang="ts">
	import Registration from '$lib/components/Registration.svelte';
	import {WebSocketTransport} from '$lib/client/transport';
	import Client from '$lib/client/client';
	import Chat from '$lib/components/chat/Chat.svelte';
	import MediumSelector from '$lib/components/medium_selector/MediumSelector.svelte';
	import Peers from '$lib/components/Peers.svelte';
	import Player from '$lib/components/player/Player.svelte';
	import {page} from '$app/state';
	import {browser} from '$app/environment';
	import {RESTClient} from '$lib/client/RESTClient';
	import {SvelteURL} from 'svelte/reactivity';
	import {registeredClient} from '$lib/stores';

	const transport = new WebSocketTransport(determineWebSocketURL());
	const restClient = new RESTClient(determineAPIURL());
	const client = new Client(transport, restClient);

	function determineWebSocketURL(): URL {
		// Just a stopgap measure for now. The (generally wrong) assumption is that
		// the backend listens on port 8000. But this is good enough for now because it
		// works both with `npm run watch` and the default backend settings as well
		// as when the frontend is bundled with the backend.
		const url = new SvelteURL(`ws://${pageHost()}/ws`);
		url.port = '8000';

		return url;
	}

	function determineAPIURL(): URL {
		// Just a stopgap measure for now. The (generally wrong) assumption is that
		// the backend listens on port 8000. But this is good enough for now because it
		// works both with `npm run watch` and the default backend settings as well
		// as when the frontend is bundled with the backend.
		const url = new SvelteURL(`http://${pageHost()}/api`);
		url.port = '8000';

		return url;
	}

	function pageHost(): string {
		if (browser) {
			return new URL(window.location.href).host;
		} else {
			return page.url.host;
		}
	}
</script>

<svelte:head>
	<title>Communityvi</title>
</svelte:head>

<section id="registration">
	<Registration {client} />
</section>

{#if $registeredClient !== undefined}
<MediumSelector registeredClient={$registeredClient} />
{/if}

<Player />

{#if $registeredClient !== undefined}
<Peers registeredClient={$registeredClient} />
<Chat registeredClient={$registeredClient} />
{/if}

