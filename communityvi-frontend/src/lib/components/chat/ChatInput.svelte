<script lang="ts">
	import {registeredClient, errorBag} from '$lib/stores';
	import {createEventDispatcher} from 'svelte';

	$: isNotRegistered = $registeredClient === undefined;

	let message = '';
	$: isMessageEmpty = message.trim().length === 0;

	async function sendChatMessage() {
		if ($registeredClient === undefined) {
			return;
		}

		try {
			const messageToSend = message;
			dispatch('chatMessageSent', messageToSend);
			message = '';
			await $registeredClient.sendChatMessage(messageToSend);
			dispatch('chatMessageAcknowledged', messageToSend);
		} catch (error) {
			console.error('Error while sending chat message:', error);
			errorBag.reportError(new Error('Chat message sending failed!'));
		}
	}

	const dispatch = createEventDispatcher();
</script>

<form on:submit|preventDefault={sendChatMessage}>
	<input type="text" bind:value={message} disabled={isNotRegistered || undefined} />
	<input type="submit" value="Send" disabled={isNotRegistered || isMessageEmpty || undefined} />
</form>
