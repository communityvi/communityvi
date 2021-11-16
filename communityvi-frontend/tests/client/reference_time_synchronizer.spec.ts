import ReferenceTimeSynchronizer, {AlreadyRunningError} from '$lib/client/reference_time_synchronizer';
import type {Connection} from '$lib/client/connection';
import {EnrichedResponse, ResponseMetadata} from '$lib/client/connection';
import {CalledWithMock, isA, mock, mockReset} from 'jest-mock-extended';
import {ClientRequest, GetReferenceTimeRequest} from '$lib/client/request';
import type {ReferenceTimeMessage} from '$lib/client/response';
import {SuccessMessageType} from '$lib/client/response';
import TimeMock from './helper/time_mock';

describe('The reference time synchronizer', () => {
	it('should not allow starting synchronization twice', async () => {
		const connectionMock = mock<Connection>();
		scheduleReferenceTimeResponse(connectionMock, 1337, 0, 0);

		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(
			connectionMock,
		);

		referenceTimeSynchronizer.start(jest.fn());

		expect(() => referenceTimeSynchronizer.start(jest.fn())).toThrowError(AlreadyRunningError);
	});

	it('should have the correct initial offset after construction', async () => {
		const connectionMock = mock<Connection>();
		scheduleReferenceTimeResponse(connectionMock, 1337, 0, 1000);

		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(
			connectionMock,
		);

		expect(referenceTimeSynchronizer.offset).toBe(1337);
	});

	it('should inform its subscriber about offset updates', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const connectionMock = mock<Connection>();
			scheduleReferenceTimeResponse(connectionMock, 1337, 0, 1000);
			const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(
				connectionMock,
			);

			const subscriber = jest.fn();
			referenceTimeSynchronizer.start(subscriber);

			// 15s passed and server and client are out of sync by 230ms.
			const outOfSyncMilliseconds = 230;
			scheduleReferenceTimeResponse(connectionMock, 16_337 + outOfSyncMilliseconds, 15_000, 16_000);
			await timeMock.advanceTimeByMilliseconds(15_000);

			expect(setInterval).toHaveBeenCalledTimes(1);
			expect(subscriber).toHaveBeenCalledWith(outOfSyncMilliseconds);
		});
	});

	it('should not inform its subscriber if the offset stays the same', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const connectionMock = mock<Connection>();
			scheduleReferenceTimeResponse(connectionMock, 1337, 0, 1000);
			const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(
				connectionMock,
			);

			const subscriber = jest.fn();
			referenceTimeSynchronizer.start(subscriber);

			// 15s passed and server and client are in sync.
			scheduleReferenceTimeResponse(connectionMock, 16_337, 15_000, 16_000);
			await timeMock.advanceTimeByMilliseconds(15_000);

			expect(setInterval).toHaveBeenCalledTimes(1);
			expect(subscriber).not.toHaveBeenCalled();
		});
	});
});

function scheduleReferenceTimeResponse(
	mock: ConnectionMock,
	referenceTime: number,
	sentAt: number,
	receivedAt: number,
) {
	mockReset(mock);
	mock.performRequest.calledWith(isA(GetReferenceTimeRequest)).mockResolvedValueOnce(
		new EnrichedResponse(
			<ReferenceTimeMessage>{
				type: SuccessMessageType.ReferenceTime,
				milliseconds: referenceTime + (receivedAt - sentAt) / 2,
			},
			new ResponseMetadata(sentAt, receivedAt),
		),
	);
}

type ConnectionMock = {performRequest: CalledWithMock<Promise<EnrichedResponse>, [ClientRequest]>} & Connection;
