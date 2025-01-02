<script lang="ts">
	import { preventDefault } from 'svelte/legacy';

	import {registeredClient, notifications} from '$lib/stores';
	import {createEventDispatcher} from 'svelte';

	let isNotRegistered = $derived($registeredClient === undefined);

	let message = $state('');
	let isMessageEmpty = $derived(message.trim().length === 0);

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
			notifications.reportError(new Error('Chat message sending failed!'));
		}
	}

	const dispatch = createEventDispatcher();
</script>

<form onsubmit={preventDefault(sendChatMessage)}>
	<input type="text" bind:value={message} disabled={isNotRegistered || undefined} />
	<input type="submit" value="Send" disabled={isNotRegistered || isMessageEmpty || undefined} />
</form>
