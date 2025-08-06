<script lang="ts">
	import ChatInput from '$lib/components/chat/ChatInput.svelte';
	import {ChatMessage} from '$lib/client/model';
	import {OwnMessage} from '$lib/components/chat/own_message';
	import {onDestroy} from 'svelte';
	import SingleChatMessage from '$lib/components/chat/SingleChatMessage.svelte';
	import RegisteredClient from '$lib/client/registered_client';

	interface Properties {
		registeredClient: RegisteredClient;
	}

	let {registeredClient}: Properties = $props();

	let messages = $state(new Array<OwnMessage | ChatMessage>());

	// NOTE: Can't use $derived because we need the side-effect of subscribing to chat messages
	// eslint-disable-next-line svelte/prefer-writable-derived
	let unsubscribe: (() => void) = $state(() => {});
	$effect(() => {
		unsubscribe = registeredClient.subscribeToChatMessages(onChatMessageReceived);
	});

	onDestroy(() => {
		unsubscribe();
	});

	function onChatMessageReceived(message: ChatMessage) {
		messages = [...messages, message];
	}

	function onChatMessageSent(messageEvent: CustomEvent) {
		const message = messageEvent.detail as string;
		messages = [...messages, new OwnMessage(message, registeredClient.asPeer())];
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
