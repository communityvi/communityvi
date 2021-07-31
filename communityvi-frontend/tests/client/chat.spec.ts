import RegisteredClient, {DisconnectCallback} from '$lib/client/registered_client';
import type ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';
import {isA, mock} from 'jest-mock-extended';
import {ChatMessage, Peer, VersionedMedium} from '$lib/client/model';
import type {Connection, ConnectionDelegate} from '$lib/client/connection';
import {ChatRequest} from '$lib/client/request';
import {BroadcastType, ChatBroadcast} from '$lib/client/broadcast';
import {EnrichedResponse, ResponseMetadata} from '$lib/client/connection';
import {SuccessMessage, SuccessMessageType} from '$lib/client/response';

describe('The registered client chat', () => {
	const versionedMedium = new VersionedMedium(0);

	it('can send chat messages', async () => {
		const mockConnection = mock<Connection>();
		const client = new RegisteredClient(
			42,
			'test_client',
			mock<ReferenceTimeSynchronizer>(),
			versionedMedium,
			[],
			mockConnection,
			mock<DisconnectCallback>(),
		);
		mockConnection.performRequest.calledWith(isA(ChatRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<SuccessMessage>{
					type: SuccessMessageType.Success,
				},
				new ResponseMetadata(0, 0),
			),
		);

		await client.sendChatMessage('hello');

		expect(mockConnection.performRequest).toHaveBeenCalledWith(new ChatRequest('hello'));
	});

	it('forwards chat messages', () => {
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = new RegisteredClient(
			42,
			'test_client',
			mock<ReferenceTimeSynchronizer>(),
			versionedMedium,
			[],
			mockConnection,
			mock<DisconnectCallback>(),
		);
		const chatMessageCallback = jest.fn();
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
		const client = new RegisteredClient(
			42,
			'test_client',
			mock<ReferenceTimeSynchronizer>(),
			versionedMedium,
			[],
			mockConnection,
			mock<DisconnectCallback>(),
		);
		const chatMessageCallback = jest.fn();
		client.subscribeToChatMessages(chatMessageCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ChatBroadcast>{
			type: BroadcastType.Chat,
			sender_id: 42,
			sender_name: 'test_client',
			message: 'irrelevant',
			counter: 1234,
		});

		expect(chatMessageCallback).not.toHaveBeenCalled();
	});
});
