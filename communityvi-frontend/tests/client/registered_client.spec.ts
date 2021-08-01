import {RegisteredClientBuilder} from './helper/registered_client_builder';
import {Peer} from '$lib/client/model';
import type {Connection, ConnectionDelegate} from '$lib/client/connection';
import {CloseReason} from '$lib/client/connection';
import {mock} from 'jest-mock-extended';
import type {BroadcastMessage, BroadcastType} from '$lib/client/broadcast';
import {UnknownBroadcastError} from '$lib/client/registered_client';
import type ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';

describe('The registered client', () => {
	it('can represent itself as a Peer', () => {
		const client = RegisteredClientBuilder.default().id(42).name('Rob').build();

		const peer = client.asPeer();

		expect(peer).toEqual(new Peer(42, 'Rob'));
	});

	it('disconnects when logging out', () => {
		const connectionMock = mock<Connection>();
		const client = RegisteredClientBuilder.default().connection(connectionMock).build();

		client.logout();

		expect(connectionMock.disconnect).toHaveBeenCalled();
	});

	it('throws an error when it receives an unknown broadcast', () => {
		const connectionMock = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		connectionMock.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		RegisteredClientBuilder.default().connection(connectionMock).build();

		const unknownBroadcastMessage = <BroadcastMessage>{
			// We must force the compiler to accept that this is an invalid broadcast type to cause the error.
			type: 'stranger_things' as BroadcastType,
		};
		expect(() => connectionDelegate?.connectionDidReceiveBroadcast(unknownBroadcastMessage)).toThrowError(
			new UnknownBroadcastError(unknownBroadcastMessage),
		);
	});

	it('calls the disconnect callback when the connection is lost or closed', () => {
		const disconnectCallback = jest.fn();

		const connectionMock = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		connectionMock.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		RegisteredClientBuilder.default().connection(connectionMock).disconnectCallback(disconnectCallback).build();

		connectionDelegate?.connectionDidClose(CloseReason.CLIENT_LEFT);

		expect(disconnectCallback).toHaveBeenCalledWith(CloseReason.CLIENT_LEFT);
	});

	it('stops the reference time synchronizer when the connection is lost or closed', () => {
		const referenceTimeSynchronizerMock = mock<ReferenceTimeSynchronizer>();

		const connectionMock = mock<Connection>();
		let connectionDelegate: ConnectionDelegate | undefined;
		connectionMock.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
		RegisteredClientBuilder.default()
			.referenceTimeSynchronizer(referenceTimeSynchronizerMock)
			.connection(connectionMock)
			.build();

		connectionDelegate?.connectionDidClose(CloseReason.CLIENT_LEFT);

		expect(referenceTimeSynchronizerMock.stop).toHaveBeenCalled();
	});
});
