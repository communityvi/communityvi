import ReferenceTimeSynchronizer, {AlreadyRunningError} from '$lib/client/reference_time_synchronizer';
import type {Connection} from '$lib/client/connection';
import {EnrichedResponse, ResponseMetadata} from '$lib/client/connection';
import {isA, mock, mockReset} from 'jest-mock-extended';
import {GetReferenceTimeRequest} from '$lib/client/request';
import type {ReferenceTimeMessage} from '$lib/client/response';
import {SuccessMessageType} from '$lib/client/response';

describe('The reference time synchronizer', () => {
	it('should not allow starting synchronization twice', async () => {
		const connectionMock = mock<Connection>();
		connectionMock.performRequest.calledWith(isA(GetReferenceTimeRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<ReferenceTimeMessage>{
					type: SuccessMessageType.ReferenceTime,
					milliseconds: 1337,
				},
				new ResponseMetadata(0, 0),
			),
		);

		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(connectionMock);

		referenceTimeSynchronizer.start(jest.fn());

		expect(() => referenceTimeSynchronizer.start(jest.fn())).toThrowError(AlreadyRunningError);
	});

	it('should have the correct initial offset after construction', async () => {
		const connectionMock = mock<Connection>();
		connectionMock.performRequest.calledWith(isA(GetReferenceTimeRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<ReferenceTimeMessage>{
					type: SuccessMessageType.ReferenceTime,
					milliseconds: 1337 + (1000 - 0) / 2,
				},
				new ResponseMetadata(0, 1000),
			),
		);

		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(connectionMock);

		expect(referenceTimeSynchronizer.offset).toBe(1337);
	});

	it('should inform its subscriber about offset updates', async () => {
		jest.useFakeTimers();

		const connectionMock = mock<Connection>();
		connectionMock.performRequest.calledWith(isA(GetReferenceTimeRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<ReferenceTimeMessage>{
					type: SuccessMessageType.ReferenceTime,
					milliseconds: 1337 + (1000 - 0) / 2,
				},
				new ResponseMetadata(0, 1000),
			),
		);
		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(connectionMock);

		const subscriber = jest.fn();
		referenceTimeSynchronizer.start(subscriber);

		// 15s passed and server and client are out of sync by 230ms.
		mockReset(connectionMock);
		connectionMock.performRequest.calledWith(isA(GetReferenceTimeRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<ReferenceTimeMessage>{
					type: SuccessMessageType.ReferenceTime,
					milliseconds: 16_337 + (16_000 - 15_000) / 2 + 230,
				},
				new ResponseMetadata(15_000, 16_000),
			),
		);
		jest.advanceTimersByTime(15_000);
		await flushPromises();

		expect(setInterval).toHaveBeenCalledTimes(1);
		expect(subscriber).toHaveBeenCalled();

		// FIXME: Has to be _cleared_ for the hack below to work!
		jest.clearAllTimers();
	});

	it('should not inform its subscriber if the offset stays the same', async () => {
		jest.useFakeTimers();

		const connectionMock = mock<Connection>();
		connectionMock.performRequest.calledWith(isA(GetReferenceTimeRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<ReferenceTimeMessage>{
					type: SuccessMessageType.ReferenceTime,
					milliseconds: 1337 + (1000 - 0) / 2,
				},
				new ResponseMetadata(0, 1000),
			),
		);
		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithConnection(connectionMock);

		const subscriber = jest.fn();
		referenceTimeSynchronizer.start(subscriber);

		// 15s passed and server and client are in sync.
		mockReset(connectionMock);
		connectionMock.performRequest.calledWith(isA(GetReferenceTimeRequest)).mockResolvedValueOnce(
			new EnrichedResponse(
				<ReferenceTimeMessage>{
					type: SuccessMessageType.ReferenceTime,
					milliseconds: 16_337 + (16_000 - 15_000) / 2,
				},
				new ResponseMetadata(15_000, 16_000),
			),
		);
		jest.advanceTimersByTime(15_000);
		await flushPromises();

		expect(setInterval).toHaveBeenCalledTimes(1);
		expect(subscriber).not.toHaveBeenCalled();

		// FIXME: Has to be _cleared_ for the hack below to work!
		jest.clearAllTimers();
	});
});

// FIXME: This is blunt _hack_ because Jest does not play nice with async/await when using fake timers.
// See: https://stackoverflow.com/questions/52177631/jest-timer-and-promise-dont-work-well-settimeout-and-async-function
function flushPromises() {
	return new Promise(resolve => setImmediate(resolve));
}
