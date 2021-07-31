import {LeaveReason, Peer, PeerJoinedMessage, PeerLeftMessage, VersionedMedium} from '$lib/client/model';
import {CalledWithMock, mock} from 'jest-mock-extended';
import type {Connection, ConnectionDelegate} from '$lib/client/connection';
import RegisteredClient, {DisconnectCallback} from '$lib/client/registered_client';
import type {ClientJoinedBroadcast, ClientLeftBroadcast} from '$lib/client/broadcast';
import {BroadcastType, LeftReason} from '$lib/client/broadcast';
import type ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';

describe('The registered client peer tracking', () => {
	it('adds joined peers to the list of peers', () => {
		const existingPeer = new Peer(0, 'existing');
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = registeredClient(mockConnection, [existingPeer]);

		connectionDelegate?.connectionDidReceiveBroadcast(<ClientJoinedBroadcast>{
			type: BroadcastType.ClientJoined,
			id: 1,
			name: 'joined',
		});

		expect(client.peers).toEqual([existingPeer, new Peer(1, 'joined')]);
	});

	it('notifies subscribers about joined peers', () => {
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = registeredClient(mockConnection, []);
		const peerLifecycleCallback = jest.fn();
		client.subscribeToPeerChanges(peerLifecycleCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ClientJoinedBroadcast>{
			type: BroadcastType.ClientJoined,
			id: 0,
			name: 'joined',
		});

		expect(peerLifecycleCallback).toHaveBeenCalledWith(new PeerJoinedMessage(new Peer(0, 'joined')));
	});

	it('removes left peers from the list of peers', () => {
		const existingPeer = new Peer(0, 'existing');
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = registeredClient(mockConnection, [existingPeer]);

		connectionDelegate?.connectionDidReceiveBroadcast(<ClientLeftBroadcast>{
			type: BroadcastType.ClientLeft,
			id: 0,
			name: 'left',
			reason: LeftReason.Closed,
		});

		expect(client.peers).toEqual([]);
	});

	it('notifies subscribers about left peers', () => {
		const existingPeer = new Peer(0, 'existing');
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = registeredClient(mockConnection, [existingPeer]);
		const peerLifecycleCallback = jest.fn();
		client.subscribeToPeerChanges(peerLifecycleCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ClientLeftBroadcast>{
			type: BroadcastType.ClientLeft,
			id: 0,
			name: 'left',
			reason: LeftReason.Closed,
		});

		expect(peerLifecycleCallback).toHaveBeenCalledWith(new PeerLeftMessage(new Peer(0, 'left'), LeaveReason.Closed));
	});

	it('does not notify subscribers if an unknown peer left', () => {
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = registeredClient(mockConnection, []);
		const peerLifecycleCallback = jest.fn();
		client.subscribeToPeerChanges(peerLifecycleCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ClientLeftBroadcast>{
			type: BroadcastType.ClientLeft,
			id: 0,
			name: 'left',
			reason: LeftReason.Closed,
		});

		expect(peerLifecycleCallback).not.toHaveBeenCalled();
	});
});

function registeredClient(mockConnection: ConnectionMock, peers: Array<Peer>): RegisteredClient {
	const versionedMedium = new VersionedMedium(0);
	return new RegisteredClient(
		42,
		'test_client',
		mock<ReferenceTimeSynchronizer>(),
		versionedMedium,
		peers,
		mockConnection,
		mock<DisconnectCallback>(),
	);
}

type ConnectionMock = {setDelegate: CalledWithMock<void, [ConnectionDelegate]>} & Connection;
