<script lang="ts">
	export let message: string;
	export let senderName: string;
	export let acknowledged: boolean | undefined = undefined;
</script>

<tr class:pendingMessage={acknowledged === false}>
	<td>{senderName}</td>
	<td>{message}</td>
</tr>

<style lang="sass">
	.pendingMessage
		color: gray
</style>
