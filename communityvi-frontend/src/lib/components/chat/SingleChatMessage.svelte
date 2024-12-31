<script lang="ts">
	import type {Peer} from '$lib/client/model';

	interface Props {
		message: string;
		sender: Peer;
		acknowledged?: boolean | undefined;
	}

	let { message, sender, acknowledged = undefined }: Props = $props();
</script>

<tr class:pendingMessage={acknowledged === false}>
	<td>{sender.name}</td>
	<td>{message}</td>
</tr>

<style lang="sass">
	.pendingMessage
		color: gray
</style>
