<script lang="ts">
	import type {Peer} from '$lib/client/model';

	export let message: string;
	export let sender: Peer;
	export let acknowledged: boolean | undefined = undefined;
</script>

<tr class:pendingMessage={acknowledged === false}>
	<td>{sender.name}</td>
	<td>{message}</td>
</tr>

<style lang="sass">
	.pendingMessage
		color: gray
</style>
