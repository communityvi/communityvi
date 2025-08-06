<script lang="ts">
	import {notifications} from '$lib/stores';
	import {Peer, PeerJoinedMessage, PeerLeftMessage} from '$lib/client/model';
	import type {PeerLifecycleMessage} from '$lib/client/model';
	import {onDestroy} from 'svelte';
	import RegisteredClient from '$lib/client/registered_client';

	interface Properties {
		registeredClient: RegisteredClient;
	}

	let {registeredClient}: Properties = $props();

	let peers: Peer[] = $derived([registeredClient.asPeer(), ...registeredClient.peers]);
	// NOTE: Can't use $derived because we need the side-effect of subscribing to peer changes
	// eslint-disable-next-line svelte/prefer-writable-derived
	let unsubscribe: (() => void) = $state(() => {});
	$effect(() => {
		unsubscribe = registeredClient.subscribeToPeerChanges(onPeerChange);
	});

	onDestroy(() => {
		unsubscribe();
	});

	function onPeerChange(peerChange: PeerLifecycleMessage) {
		if (peerChange instanceof PeerJoinedMessage) {
			peers = [...peers, peerChange.peer];
			notifications.inform(`'${peerChange.peer.name}' joined.`);
		} else if (peerChange instanceof PeerLeftMessage) {
			peers = [registeredClient.asPeer(), ...registeredClient.peers];
			notifications.inform(`'${peerChange.peer.name}' left.`);
		}
	}
</script>

<table class="table">
	<thead>
		<tr>
			<th>Peers</th>
		</tr>
	</thead>
	<tbody>
		{#each peers as peer (peer.id)}
			<tr>
				<td>{peer.name}</td>
			</tr>
		{/each}
	</tbody>
</table>

<style lang="sass">
tbody tr
	font-weight: bold
	&:first-child
		font-style: italic
</style>
