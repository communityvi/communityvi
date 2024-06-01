import {isA, mock} from 'vitest-mock-extended';
import {ChatMessage, Peer} from '$lib/client/model';
import type {Connection, ConnectionDelegate} from '$lib/client/connection';
import {ChatRequest} from '$lib/client/request';
import {BroadcastType, ChatBroadcast} from '$lib/client/broadcast';
import {EnrichedResponse, ResponseMetadata} from '$lib/client/connection';
import {SuccessMessage, SuccessMessageType} from '$lib/client/response';
import {RegisteredClientBuilder} from './helper/registered_client_builder';
import {describe, it, expect, vi} from 'vitest';

describe('The registered client chat', () => {
	it('can send chat messages', async () => {
		const mockConnection = mock<Connection>();
		mockConnection.performRequest.calledWith(isA(ChatRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<SuccessMessage>{
					type: SuccessMessageType.Success,
				},
				new ResponseMetadata(0, 0),
			),
		);
		const client = RegisteredClientBuilder.default().connection(mockConnection).build();

		await client.sendChatMessage('hello');

		expect(mockConnection.performRequest).toHaveBeenCalledWith(new ChatRequest('hello'));
	});

	it('forwards chat messages', () => {
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = RegisteredClientBuilder.default().id(0).connection(mockConnection).build();
		const chatMessageCallback = vi.fn();
		client.subscribeToChatMessages(chatMessageCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ChatBroadcast>{
			type: BroadcastType.Chat,
			sender_id: 1337,
			sender_name: 'peer',
			message: 'hello',
			counter: 1234,
		});

		expect(chatMessageCallback).toHaveBeenCalledWith(new ChatMessage('hello', new Peer(1337, 'peer')));
	});

	it('ignores chat messages from itself', () => {
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = RegisteredClientBuilder.default().connection(mockConnection).build();
		const chatMessageCallback = vi.fn();
		client.subscribeToChatMessages(chatMessageCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ChatBroadcast>{
			type: BroadcastType.Chat,
			sender_id: client.id,
			sender_name: client.name,
			message: 'irrelevant',
			counter: 1234,
		});

		expect(chatMessageCallback).not.toHaveBeenCalled();
	});
});
