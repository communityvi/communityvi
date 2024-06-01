export function promiseWithTimout<T>(
	promise: Promise<T>,
	timeoutInMilliseconds: number,
	timeoutHandler: () => void,
): Promise<T> {
	let timeoutId: number;

	const timeoutPromise = new Promise<never>((resolve, reject) => {
		timeoutId = setTimeout(() => {
			timeoutHandler();
			reject(new TimeoutError(timeoutInMilliseconds));
		}, timeoutInMilliseconds);
	});

	return Promise.race([promise, timeoutPromise]).then(result => {
		clearTimeout(timeoutId);
		return result;
	});
}

class TimeoutError extends Error {
	readonly timeoutInMilliseconds: number;

	constructor(timeoutInMilliseconds: number) {
		super(`Promise did not fulfill in ${timeoutInMilliseconds} ms.`);

		this.name = TimeoutError.name;
		this.timeoutInMilliseconds = timeoutInMilliseconds;
	}
}
