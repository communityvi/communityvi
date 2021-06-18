<script lang="ts">
	import {registeredClient, errorBag} from '$lib/stores';

	$: isNotRegistered = $registeredClient === undefined;

	let message = '';
	$: isMessageEmpty = message.trim().length === 0;

	async function sendChatMessage() {
		try {
			await $registeredClient.sendChatMessage(message);
			message = '';
		} catch (error) {
			console.error('Error while sending chat message:', error);
			errorBag.reportError(new Error('Chat message sending failed!'));
		}
	}
</script>

<form on:submit|preventDefault={sendChatMessage}>
	<input type="text" bind:value={message} disabled={isNotRegistered || undefined} />
	<input type="submit" value="Send" disabled={isNotRegistered || isMessageEmpty || undefined} />
</form>
