<script lang="ts">
	interface Properties {
		onNewMessage: (message: string) => void;
	}

	let {onNewMessage}: Properties = $props();

	let message = $state('');
	let isMessageEmpty = $derived(message.trim().length === 0);

	let textInput: HTMLInputElement;

	async function sendChatMessage() {
		onNewMessage(message)
		message = '';
		textInput.focus();
	}
</script>

<form onsubmit={sendChatMessage}>
	<input type="text" bind:this={textInput} bind:value={message} />
	<input type="submit" value="Send" disabled={isMessageEmpty} />
</form>
