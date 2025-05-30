<script lang="ts">
	import ChatInput from '$lib/components/chat/ChatInput.svelte';
	import {ChatMessage} from '$lib/client/model';
	import {registeredClient} from '$lib/stores';
	import {OwnMessage} from '$lib/components/chat/own_message';
	import {onDestroy} from 'svelte';
	import SingleChatMessage from '$lib/components/chat/SingleChatMessage.svelte';

	let messages = $state(new Array<OwnMessage | ChatMessage>());

	// NOTE: Can't use $derived because we need the side-effect of subscribing to chat messages
	// eslint-disable-next-line svelte/prefer-writable-derived
	let unsubscribe: (() => void) | undefined = $state(undefined);
	$effect(() => {
		unsubscribe = $registeredClient?.subscribeToChatMessages(onChatMessageReceived);
	});

	onDestroy(() => {
		if (unsubscribe === undefined) {
			return;
		}

		unsubscribe();
	});

	function onChatMessageReceived(message: ChatMessage) {
		messages = [...messages, message];
	}

	function onChatMessageSent(messageEvent: CustomEvent) {
		if ($registeredClient === undefined) {
			return;
		}

		const message = messageEvent.detail as string;
		messages = [...messages, new OwnMessage(message, $registeredClient.asPeer())];
	}

	function onChatMessageAcknowledged(acknowledgedEvent: CustomEvent) {
		const message = acknowledgedEvent.detail as string;
		// Array.map is used here because svelte needs an assignment to message to trigger a DOM update
		messages = messages.map(existingMessage => {
			if (!(existingMessage instanceof OwnMessage)) {
				return existingMessage;
			}

			if (existingMessage.acknowledged) {
				return existingMessage;
			}

			if (existingMessage.message !== message) {
				return existingMessage;
			}

			existingMessage.acknowledged = true;
			return existingMessage;
		});
	}
</script>

<section id="chat" class="table-container">
	<table class="table">
		<thead>
			<tr>
				<th>Name</th>
				<th>Message</th>
			</tr>
		</thead>

		<tbody>
			<!-- TODO: Look into whether the key is unique enough or whether we need an additional ID -->
			{#each messages as message ([message.sender.id, message.message])}
				{#if message instanceof ChatMessage}
					<SingleChatMessage message={message.message} sender={message.sender} />
				{:else if message instanceof OwnMessage}
					<SingleChatMessage
						message={message.message}
						sender={message.sender}
						acknowledged={message.acknowledged}
					/>
				{/if}
			{/each}
		</tbody>
	</table>
	<ChatInput on:chatMessageSent={onChatMessageSent} on:chatMessageAcknowledged={onChatMessageAcknowledged} />
</section>
