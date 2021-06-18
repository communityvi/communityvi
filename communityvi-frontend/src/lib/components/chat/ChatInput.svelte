<script lang="ts">
	import {registeredClient} from '$lib/stores';

	$: isNotRegistered = $registeredClient === undefined;

	let message = '';
	$: isMessageEmpty = message.trim().length === 0;

	function sendChatMessage() {
		$registeredClient.sendChatMessage(message);
		message = '';
	}
</script>

<form on:submit|preventDefault={sendChatMessage}>
	<input type="text" bind:value={message} disabled={isNotRegistered || undefined} />
	<input type="submit" value="Send" disabled={isNotRegistered || isMessageEmpty || undefined} />
</form>
