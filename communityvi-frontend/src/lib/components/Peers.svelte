<script lang="ts">
	import {registeredClient} from '$lib/stores';
	import {PeerJoinedMessage, PeerLeftMessage} from '$lib/client/model';
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

		if (peerChange instanceof PeerJoinedMessage) {
			peers = [...peers, peerChange.peer];
			// TODO: Notification
		} else if (peerChange instanceof PeerLeftMessage) {
			peers = [$registeredClient.asPeer(), ...$registeredClient.peers];
			// TODO: Notification
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
