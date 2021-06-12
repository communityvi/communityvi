import type {Readable, Subscriber, Unsubscriber} from 'svelte/store';

export class ErrorBag implements Readable<IdentifiableErrors> {
	private count = 0;
	private errors: Array<IdentifiableError> = [];

	private readonly subscribers = new Array<Subscriber<IdentifiableErrors>>();

	reportError(error: Error): void {
		this.errors.push({
			id: ++this.count,
			error: error,
		});

		this.notifyAllSubscribers();
	}

	deleteWithId(id: number): void {
		this.errors = this.errors.filter(error => error.id !== id);

		this.notifyAllSubscribers();
	}

	private notifyAllSubscribers(): void {
		const errors = this.copiedErrors();
		this.subscribers.forEach(subscriber => {
			subscriber(errors);
		});
	}

	subscribe(subscriber: Subscriber<IdentifiableErrors>): Unsubscriber {
		this.subscribers.push(subscriber);

		subscriber(this.copiedErrors());

		return () => {
			const index = this.subscribers.indexOf(subscriber);
			if (index === -1) {
				return;
			}

			this.subscribers.splice(index, 1);
		};
	}

	private copiedErrors(): IdentifiableErrors {
		return [...this.errors];
	}
}

type IdentifiableErrors = Array<IdentifiableError>;

export interface IdentifiableError {
	readonly id: number;
	readonly error: Error;
}
