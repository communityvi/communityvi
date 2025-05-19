<script lang="ts">
	import {registeredClient, notifications} from '$lib/stores';
	import {createEventDispatcher} from 'svelte';

	let isNotRegistered = $derived($registeredClient === undefined);

	let message = $state('');
	let isMessageEmpty = $derived(message.trim().length === 0);

	let textInput: HTMLInputElement | undefined = $state(undefined);

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

			textInput?.focus();
		} catch (error) {
			console.error('Error while sending chat message:', error);
			notifications.reportError(new Error('Chat message sending failed!'));
		}
	}

	const dispatch = createEventDispatcher();
</script>

<form onsubmit={sendChatMessage}>
	<input type="text" bind:this={textInput} bind:value={message} disabled={isNotRegistered || undefined} />
	<input type="submit" value="Send" disabled={isNotRegistered || isMessageEmpty || undefined} />
</form>
