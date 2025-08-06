<script lang="ts">
	import ChatInput from '$lib/components/chat/ChatInput.svelte';
	import {ChatMessage} from '$lib/client/model';
	import {OwnMessage} from '$lib/components/chat/own_message';
	import {onDestroy} from 'svelte';
	import SingleChatMessage from '$lib/components/chat/SingleChatMessage.svelte';
	import RegisteredClient from '$lib/client/registered_client';
	import {notifications} from '$lib/stores';

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

	async function onNewMessage(message: string) {
		const newMessage = new OwnMessage(message, registeredClient.asPeer());
		messages = [...messages, newMessage];

		try {
			await registeredClient.sendChatMessage(message);
		} catch (error) {
			console.error('Error while sending chat message:', error);
			notifications.reportError(new Error('Chat message sending failed!'));
			return;
		}

		newMessage.acknowledged = true;
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
	<ChatInput {onNewMessage} />
</section>
