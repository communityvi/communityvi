import type {Readable, Subscriber, Unsubscriber} from 'svelte/store';
import {NotificationType} from '$lib/components/notification/notification_type';
import MessageBroker from '$lib/client/message_broker';

export class NotificationStore implements Readable<IdentifiableNotifications> {
	private count = 0;
	private notifications: IdentifiableNotifications = [];

	private readonly messageBroker = new MessageBroker<IdentifiableNotifications>();

	private get copiedNotifications(): IdentifiableNotifications {
		return [...this.notifications];
	}

	reportError(error: Error): void {
		const notification = IdentifiableNotification.fromIdAndError(++this.count, error);
		this.notify(notification);
	}

	inform(message: string): void {
		const notification = new IdentifiableNotification(++this.count, NotificationType.PRIMARY, message);
		this.notify(notification);
	}

	error(message: string): void {
		const notification = new IdentifiableNotification(++this.count, NotificationType.DANGER, message);
		this.notify(notification);
	}

	private notify(notification: IdentifiableNotification): void {
		this.notifications.push(notification);
		this.messageBroker.notify(this.copiedNotifications);
	}

	deleteWithId(id: number): void {
		this.notifications = this.notifications.filter(notification => notification.id !== id);

		this.messageBroker.notify(this.copiedNotifications);
	}

	subscribe(subscriber: Subscriber<IdentifiableNotifications>): Unsubscriber {
		const unsubscriber = this.messageBroker.subscribe(subscriber);

		subscriber(this.copiedNotifications);

		return unsubscriber;
	}
}

type IdentifiableNotifications = Array<IdentifiableNotification>;

export class IdentifiableNotification {
	readonly id: number;
	readonly type: NotificationType;
	readonly message: string;

	static fromIdAndError(id: number, error: Error): IdentifiableNotification {
		return new IdentifiableNotification(id, NotificationType.DANGER, error.message);
	}

	constructor(id: number, type: NotificationType, message: string) {
		this.id = id;
		this.type = type;
		this.message = message;
	}
}
