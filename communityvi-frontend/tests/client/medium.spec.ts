import type {Connection, ConnectionDelegate} from '$lib/client/connection';
import {EnrichedResponse, ResponseMetadata} from '$lib/client/connection';
import {isA, mock} from 'vitest-mock-extended';
import {RegisteredClientBuilder} from './helper/registered_client_builder';
import {BroadcastType, MediumStateChangedBroadcast, VersionedMediumBroadcast} from '$lib/client/broadcast';
import {
	EmptyMedium,
	FixedLengthMedium,
	InsertMediumRequest,
	MediumType,
	PauseRequest,
	PlayRequest,
} from '$lib/client/request';
import {
	Medium,
	MediumChangedByOurself,
	MediumChangedByPeer,
	MediumTimeAdjusted,
	Peer,
	PlayingPlaybackState,
	VersionedMedium,
} from '$lib/client/model';
import type ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';
import type {TimeUpdatedCallback} from '$lib/client/reference_time_synchronizer';
import type {SuccessMessage} from '$lib/client/response';
import {SuccessMessageType} from '$lib/client/response';
import {describe, it, expect, vi} from 'vitest';

describe('RegisteredClient medium handling', () => {
	const warGames = new Medium('WarGames', 114 * 60 * 1000);
	const successResponse = new EnrichedResponse(
		<SuccessMessage>{
			type: SuccessMessageType.Success,
		},
		new ResponseMetadata(0, 0),
	);

	describe('subscriber notifications', () => {
		it('notifies subscribers when others change something about the medium', () => {
			const mockConnection = mock<Connection>();
			let connectionDelegate: ConnectionDelegate | undefined;
			mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
			const client = RegisteredClientBuilder.default().id(42).connection(mockConnection).build();
			const subscriber = vi.fn();

			client.subscribeToMediumStateChanges(subscriber);
			connectionDelegate?.connectionDidReceiveBroadcast(<MediumStateChangedBroadcast>{
				type: BroadcastType.MediumStateChanged,
				changed_by_id: 23,
				changed_by_name: 'some_other_client',
				medium: <VersionedMediumBroadcast>{
					type: MediumType.Empty,
				},
			});

			expect(subscriber).toHaveBeenCalledWith(
				new MediumChangedByPeer(new Peer(23, 'some_other_client'), undefined),
			);
		});

		it('ignores broadcasts when we have changed the medium', () => {
			const mockConnection = mock<Connection>();
			let connectionDelegate: ConnectionDelegate | undefined;
			mockConnection.setDelegate.mockImplementationOnce(delegate => (connectionDelegate = delegate));
			const client = RegisteredClientBuilder.default().connection(mockConnection).build();
			const subscriber = vi.fn();
			client.subscribeToMediumStateChanges(subscriber);

			connectionDelegate?.connectionDidReceiveBroadcast(<MediumStateChangedBroadcast>{
				type: BroadcastType.MediumStateChanged,
				changed_by_id: client.id,
				changed_by_name: client.name,
				medium: <VersionedMediumBroadcast>{
					type: MediumType.Empty,
				},
			});

			expect(subscriber).not.toHaveBeenCalled();
		});

		it('notifies subscribers when a medium is playing and the reference time got updated', () => {
			// Given we pulled out the reference time callback
			let referenceTimeUpdatedCallback: TimeUpdatedCallback | undefined;
			const referenceTimeSynchronizerMock = mock<ReferenceTimeSynchronizer>();
			referenceTimeSynchronizerMock.start.mockImplementationOnce(
				callback => (referenceTimeUpdatedCallback = callback),
			);

			// ...and we had a subscriber
			const subscriber = vi.fn();

			// ...that is subscribed to a registered client with a playing medium
			const playingVersionedMedium = new VersionedMedium(
				0,
				new Medium(warGames.name, warGames.lengthInMilliseconds, false, new PlayingPlaybackState(0)),
			);
			const client = RegisteredClientBuilder.default()
				.versionedMedium(playingVersionedMedium)
				.referenceTimeSynchronizer(referenceTimeSynchronizerMock)
				.build();
			client.subscribeToMediumStateChanges(subscriber);

			// when
			referenceTimeUpdatedCallback?.call(undefined, 123);

			// then
			expect(subscriber).toHaveBeenCalledWith(
				new MediumTimeAdjusted(
					new Medium(warGames.name, warGames.lengthInMilliseconds, false, new PlayingPlaybackState(123)),
					123,
				),
			);
		});

		it('does not notify subscribers about reference time changes when there is no medium playing', () => {
			// Given we pulled out the reference time callback
			let referenceTimeUpdatedCallback: TimeUpdatedCallback | undefined;
			const referenceTimeSynchronizerMock = mock<ReferenceTimeSynchronizer>();
			referenceTimeSynchronizerMock.start.mockImplementationOnce(
				callback => (referenceTimeUpdatedCallback = callback),
			);

			// ...and we had a subscriber
			const subscriber = vi.fn();

			// ...that is subscribed to a registered client without a medium
			const client = RegisteredClientBuilder.default()
				.referenceTimeSynchronizer(referenceTimeSynchronizerMock)
				.build();
			client.subscribeToMediumStateChanges(subscriber);

			// when
			referenceTimeUpdatedCallback?.call(undefined, 123);

			// then
			expect(subscriber).not.toHaveBeenCalled();
		});
	});

	describe('inserting media', () => {
		it('inserts new media', async () => {
			const client = RegisteredClientBuilder.default().build();

			await client.insertFixedLengthMedium(warGames.name, warGames.lengthInMilliseconds);

			expect(client.currentMedium).toEqual(warGames);
		});

		it('notifies subscribers', async () => {
			const client = RegisteredClientBuilder.default().build();
			const subscriber = vi.fn();

			client.subscribeToMediumStateChanges(subscriber);
			await client.insertFixedLengthMedium(warGames.name, warGames.lengthInMilliseconds);

			expect(subscriber).toHaveBeenCalledWith(new MediumChangedByOurself(warGames));
		});

		it('calls the backend with the appropriate information', async () => {
			const connectionMock = mock<Connection>();
			connectionMock.performRequest.calledWith(isA(InsertMediumRequest)).mockResolvedValueOnce(successResponse);
			const client = RegisteredClientBuilder.default()
				.versionedMedium(new VersionedMedium(0, undefined))
				.connection(connectionMock)
				.build();

			await client.insertFixedLengthMedium(warGames.name, warGames.lengthInMilliseconds);

			expect(connectionMock.performRequest).toHaveBeenCalledWith(
				new InsertMediumRequest(0, new FixedLengthMedium(warGames.name, warGames.lengthInMilliseconds)),
			);
		});
	});

	describe('ejecting media', () => {
		it('ejects media', async () => {
			const client = RegisteredClientBuilder.default().build();

			await client.ejectMedium();

			expect(client.currentMedium).toBe(undefined);
		});

		it('notifies subscribers', async () => {
			const client = RegisteredClientBuilder.default().build();
			const subscriber = vi.fn();

			client.subscribeToMediumStateChanges(subscriber);
			await client.ejectMedium();

			expect(subscriber).toHaveBeenCalledWith(new MediumChangedByOurself(undefined));
		});

		it('calls the backend with the appropriate information', async () => {
			const connectionMock = mock<Connection>();
			connectionMock.performRequest.calledWith(isA(InsertMediumRequest)).mockResolvedValueOnce(successResponse);
			const client = RegisteredClientBuilder.default()
				.versionedMedium(new VersionedMedium(0, warGames))
				.connection(connectionMock)
				.build();

			await client.ejectMedium();

			expect(connectionMock.performRequest).toHaveBeenCalledWith(new InsertMediumRequest(0, new EmptyMedium()));
		});
	});

	describe('playing/pausing media', () => {
		it('sends request to play medium', async () => {
			const connectionMock = mock<Connection>();
			connectionMock.performRequest.calledWith(isA(PlayRequest)).mockResolvedValueOnce(successResponse);
			const client = RegisteredClientBuilder.default()
				.versionedMedium(new VersionedMedium(0, undefined))
				.connection(connectionMock)
				.build();

			await client.play(0, true);

			expect(connectionMock.performRequest).toHaveBeenCalledWith(new PlayRequest(0, true, 0));
		});

		it('sends request to pause medium', async () => {
			const connectionMock = mock<Connection>();
			connectionMock.performRequest.calledWith(isA(PauseRequest)).mockResolvedValueOnce(successResponse);
			const client = RegisteredClientBuilder.default()
				.versionedMedium(new VersionedMedium(0, undefined))
				.connection(connectionMock)
				.build();

			await client.pause(0, true);

			expect(connectionMock.performRequest).toHaveBeenCalledWith(new PauseRequest(0, true, 0));
		});
	});
});
