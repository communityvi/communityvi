export default class RateLimiter {
	private readonly intervalInMilliseconds: number;

	private timeOfLastCall: DOMTimeStamp | DOMHighResTimeStamp;
	private pendingTimeout?: NodeJS.Timeout;

	constructor(intervalInMilliseconds: number) {
		this.intervalInMilliseconds = intervalInMilliseconds;
		this.timeOfLastCall = performance.now() - intervalInMilliseconds;
	}

	// Schedule a call to be called, limited by the configured rate limit.
	// NOTE: If calls are coming in too fast, only the last one is guaranteed to happen.
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
