export default class MessageBroker<MessageType> {
	private readonly subscribers = new Array<Subscriber<MessageType>>();

	subscribe(subscriber: Subscriber<MessageType>): Unsubscriber {
		this.subscribers.push(subscriber);

		return () => {
			const index = this.subscribers.indexOf(subscriber);
			if (index === -1) {
				return;
			}

			this.subscribers.splice(index, 1);
		};
	}

	notify(message: MessageType): void {
		for (const callback of this.subscribers) {
			callback(message);
		}
	}
}

export type Subscriber<MessageType> = (message: MessageType) => void;
export type Unsubscriber = () => void;
