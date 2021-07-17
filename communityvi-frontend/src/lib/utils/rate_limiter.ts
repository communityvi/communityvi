export default class RateLimiter {
	private readonly intervalInMilliseconds: number;

	private timeOfLastCall: DOMTimeStamp | DOMHighResTimeStamp;
	private pendingTimeout?: NodeJS.Timeout;

	constructor(intervalInMilliseconds: number) {
		this.intervalInMilliseconds = intervalInMilliseconds;
		this.timeOfLastCall = performance.now() - intervalInMilliseconds;
	}

	call(call: () => void): void {
		const callAndUpdateTimeOfLastCall = () => {
			call();
			this.timeOfLastCall = performance.now();
		};

		if (performance.now() - this.timeOfLastCall >= this.intervalInMilliseconds) {
			this.replacePendingTimeout();
			callAndUpdateTimeOfLastCall();
			return;
		}

		const timeout = setTimeout(callAndUpdateTimeOfLastCall, this.intervalInMilliseconds);
		this.replacePendingTimeout(timeout);
	}

	reset(): void {
		this.replacePendingTimeout();
	}

	private replacePendingTimeout(newTimeout?: NodeJS.Timeout) {
		if (this.pendingTimeout !== undefined) {
			clearTimeout(this.pendingTimeout);
		}

		this.pendingTimeout = newTimeout;
	}
}
