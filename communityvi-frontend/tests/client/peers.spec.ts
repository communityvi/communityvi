import {LeaveReason, Peer, PeerJoinedMessage, PeerLeftMessage} from '$lib/client/model';
import {mock} from 'jest-mock-extended';
import type {Connection, ConnectionDelegate} from '$lib/client/connection';
import type {ClientJoinedBroadcast, ClientLeftBroadcast} from '$lib/client/broadcast';
import {BroadcastType, LeftReason} from '$lib/client/broadcast';
import {RegisteredClientBuilder} from './helper/registered_client_builder';

describe('The registered client peer tracking', () => {
	it('adds joined peers to the list of peers', () => {
		const existingPeer = new Peer(0, 'existing');
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = RegisteredClientBuilder.default().id(42).peer(existingPeer).connection(mockConnection).build();

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
		const client = RegisteredClientBuilder.default().id(42).connection(mockConnection).build();
		const peerLifecycleCallback = jest.fn();
		client.subscribeToPeerChanges(peerLifecycleCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ClientJoinedBroadcast>{
			type: BroadcastType.ClientJoined,
			id: 0,
			name: 'joined',
		});

		expect(peerLifecycleCallback).toHaveBeenCalledWith(new PeerJoinedMessage(new Peer(0, 'joined')));
	});

	it('ignores the broadcast that we have joined ourselves', () => {
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = RegisteredClientBuilder.default().id(42).connection(mockConnection).build();
		const peerLifecycleCallback = jest.fn();
		client.subscribeToPeerChanges(peerLifecycleCallback);

		connectionDelegate?.connectionDidReceiveBroadcast(<ClientJoinedBroadcast>{
			type: BroadcastType.ClientJoined,
			id: client.id,
			name: client.name,
		});

		expect(peerLifecycleCallback).not.toHaveBeenCalled();
		expect(client.peers).toEqual([]);
	});

	it('removes left peers from the list of peers', () => {
		const existingPeer = new Peer(0, 'existing');
		const mockConnection = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		const client = RegisteredClientBuilder.default().id(42).peer(existingPeer).connection(mockConnection).build();

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
		const client = RegisteredClientBuilder.default().id(42).peer(existingPeer).connection(mockConnection).build();
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
		const client = RegisteredClientBuilder.default().id(42).connection(mockConnection).build();
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
