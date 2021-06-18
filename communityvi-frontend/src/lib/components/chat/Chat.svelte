<script lang="ts">
	import ChatInput from '$lib/components/chat/ChatInput.svelte';
	import {ChatMessage} from '$lib/client/model';
	import {registeredClient} from '$lib/stores';
	import {OwnMessage} from '$lib/components/chat/own_message';
	import {onDestroy} from 'svelte';

	let messages = new Array<OwnMessage | ChatMessage>();

	$: unsubscribe = $registeredClient?.subscribeToChatMessages(onChatMessageReceived);

	onDestroy(() => {
		if (unsubscribe !== undefined) {
			unsubscribe();
		}
	});

	function onChatMessageReceived(message: ChatMessage) {
		messages = [...messages, message];
	}

	function onChatMessageSent(messageEvent: CustomEvent) {
		if ($registeredClient === undefined) {
			return;
		}

		const message = messageEvent.detail as string;
		messages = [...messages, new OwnMessage(message, $registeredClient.id, $registeredClient.name)];
	}

	function onChatMessageAcknowledged(acknowlegdedEvent: CustomEvent) {
		const message = acknowlegdedEvent.detail as string;
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

<section id="chat">
	<table>
		<thead>
			<tr>
				<td>ID</td>
				<td>Name</td>
				<td>Message</td>
			</tr>
		</thead>

		<tbody>
			{#each messages as message}
				{#if message instanceof ChatMessage}
					<tr>
						<td>{message.senderId}</td>
						<td>{message.senderName}</td>
						<td>{message.message}</td>
					</tr>
				{:else}
					<tr class:pendingMessage={!message.acknowledged}>
						<td>{message.senderId}</td>
						<td>{message.senderName}</td>
						<td>{message.message}</td>
					</tr>
				{/if}
			{/each}
		</tbody>
	</table>
	<ChatInput on:chatMessageSent={onChatMessageSent} on:chatMessageAcknowledged={onChatMessageAcknowledged} />
</section>

<style lang="sass">
.pendingMessage
	color: gray
</style>
