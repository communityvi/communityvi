import ReferenceTimeSynchronizer, {AlreadyRunningError} from '$lib/client/reference_time_synchronizer';
import {CalledWithMock, mock, mockReset} from 'jest-mock-extended';
import TimeMock from './helper/time_mock';
import {ReferenceTimeResponse, RESTClient} from '$lib/client/RESTClient';

describe('The reference time synchronizer', () => {
	it('should not allow starting synchronization twice', async () => {
		const restClientMock = mock<RESTClient>();
		scheduleReferenceTimeResponse(restClientMock, 1337);

		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithRESTClient(
			restClientMock,
		);

		referenceTimeSynchronizer.start(jest.fn());

		expect(() => referenceTimeSynchronizer.start(jest.fn())).toThrowError(AlreadyRunningError);
	});

	it('should have the correct initial offset after construction', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const restClientMock = mock<RESTClient>();
			scheduleReferenceTimeResponse(restClientMock, 1337, timeMock, 1_000);

			const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithRESTClient(
				restClientMock,
			);

			expect(referenceTimeSynchronizer.offset).toBe(1337);
		});
	});

	it('should inform its subscriber about offset updates', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const initialReferenceTime = 1337;
			const restClientMock = mock<RESTClient>();
			scheduleReferenceTimeResponse(restClientMock, initialReferenceTime);
			const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithRESTClient(
				restClientMock,
			);

			const subscriber = jest.fn();
			referenceTimeSynchronizer.start(subscriber);

			// 15s passed and server and client are out of sync by 230ms.
			const outOfSyncMilliseconds = 230;
			const responseReferenceTime =
				ReferenceTimeSynchronizer.UPDATE_INTERVAL_MILLISECONDS + initialReferenceTime + outOfSyncMilliseconds;
			scheduleReferenceTimeResponse(restClientMock, responseReferenceTime, timeMock, 1_000);
			await timeMock.advanceTimeByMilliseconds(ReferenceTimeSynchronizer.UPDATE_INTERVAL_MILLISECONDS);

			expect(setInterval).toHaveBeenCalledTimes(1);
			expect(subscriber).toHaveBeenCalledWith(outOfSyncMilliseconds);
		});
	});

	it('should not inform its subscriber if the offset stays the same', async () => {
		await TimeMock.run(async (timeMock: TimeMock) => {
			const initialReferenceTime = 1337;
			const restClientMock = mock<RESTClient>();
			scheduleReferenceTimeResponse(restClientMock, initialReferenceTime, timeMock, 0);
			const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithRESTClient(
				restClientMock,
			);

			const subscriber = jest.fn();
			referenceTimeSynchronizer.start(subscriber);

			// 15s passed and server and client are in sync.
			const responseReferenceTime = ReferenceTimeSynchronizer.UPDATE_INTERVAL_MILLISECONDS + initialReferenceTime;
			scheduleReferenceTimeResponse(restClientMock, responseReferenceTime, timeMock, 1_000);
			await timeMock.advanceTimeByMilliseconds(ReferenceTimeSynchronizer.UPDATE_INTERVAL_MILLISECONDS);

			expect(setInterval).toHaveBeenCalledTimes(1);
			expect(subscriber).not.toHaveBeenCalled();
		});
	});
});

function scheduleReferenceTimeResponse(
	restClient: RESTClientMock,
	referenceTime: number,
	timeMock?: TimeMock,
	pingMilliseconds = 0,
) {
	mockReset(restClient);
	restClient.getReferenceTimeMilliseconds.mockImplementationOnce(async () => {
		const sentAt = performance.now();
		if (timeMock !== undefined) {
			await timeMock?.advanceTimeByMilliseconds(pingMilliseconds);
		}

		return new ReferenceTimeResponse(referenceTime + pingMilliseconds / 2, sentAt, sentAt + pingMilliseconds);
	});
}

type RESTClientMock = {getReferenceTimeMilliseconds: CalledWithMock<Promise<ReferenceTimeResponse>, []>} & RESTClient;
