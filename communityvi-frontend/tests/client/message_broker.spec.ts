import {jest} from '@jest/globals';
import MessageBroker from '$lib/client/message_broker';

describe('The message broker', () => {
	it('notifies subscribers', () => {
		const messageBroker = new MessageBroker<string>();
		const subscriber1 = jest.fn();
		const subscriber2 = jest.fn();

		messageBroker.subscribe(subscriber1);
		messageBroker.subscribe(subscriber2);

		messageBroker.notify('Hello, world!');

		expect(subscriber1).toHaveBeenCalledWith('Hello, world!');
		expect(subscriber2).toHaveBeenCalledWith('Hello, world!');
	});

	it('allows subscribers to unsubscribe themselves', () => {
		const messageBroker = new MessageBroker<string>();
		const subscriber1 = jest.fn();
		const subscriber2 = jest.fn();

		const unsubscribeSubscriber1 = messageBroker.subscribe(subscriber1);
		messageBroker.subscribe(subscriber2);

		unsubscribeSubscriber1();
		messageBroker.notify('Hello, world!');

		expect(subscriber1).not.toHaveBeenCalled();
		expect(subscriber2).toHaveBeenCalledWith('Hello, world!');
	});
});
