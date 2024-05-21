<script lang="ts">
	import {registeredClient, notifications} from '$lib/stores';
	import {PeerJoinedMessage, PeerLeftMessage, PeersRefreshedMessage} from '$lib/client/model';
	import type {PeerLifecycleMessage} from '$lib/client/model';
	import {onDestroy} from 'svelte';

	$: peers = $registeredClient && [$registeredClient.asPeer(), ...$registeredClient.peers];

	$: unsubscribe = $registeredClient?.subscribeToPeerChanges(onPeerChange);

	onDestroy(() => {
		if (unsubscribe !== undefined) {
			unsubscribe();
		}
	});

	function onPeerChange(peerChange: PeerLifecycleMessage) {
		if (peers === undefined || $registeredClient === undefined) {
			return;
		}

		if (peerChange instanceof PeersRefreshedMessage) {
			peers = [$registeredClient.asPeer(), ...peerChange.peers];
		} else if (peerChange instanceof PeerJoinedMessage) {
			peers = [...peers, peerChange.peer];
			notifications.inform(`'${peerChange.peer.name}' joined.`);
		} else if (peerChange instanceof PeerLeftMessage) {
			peers = [$registeredClient.asPeer(), ...$registeredClient.peers];
			notifications.inform(`'${peerChange.peer.name}' left.`);
		}
	}
</script>

{#if peers !== undefined}
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
{/if}

<style lang="sass">
tbody tr
	font-weight: bold
	&:first-child
		font-style: italic
</style>
