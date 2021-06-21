import type {Readable, Subscriber, Unsubscriber} from 'svelte/store';
import {NotificationType} from '$lib/components/notification/notification_type';

export class NotificationStore implements Readable<IdentifiableNotifications> {
	private count = 0;
	private notifications: Array<IdentifiableNotification> = [];

	private readonly subscribers = new Array<Subscriber<IdentifiableNotifications>>();

	reportError(error: Error): void {
		const notification = IdentifiableNotification.fromIdAndError(++this.count, error);
		this.notify(notification);
	}

	inform(message: string): void {
		const notification = new IdentifiableNotification(++this.count, NotificationType.PRIMARY, message);
		this.notify(notification);
	}

	private notify(notification: IdentifiableNotification): void {
		this.notifications.push(notification);
		this.notifyAllSubscribers();
	}

	deleteWithId(id: number): void {
		this.notifications = this.notifications.filter(notification => notification.id !== id);

		this.notifyAllSubscribers();
	}

	private notifyAllSubscribers(): void {
		const notifications = this.copiedNotifications();
		this.subscribers.forEach(subscriber => {
			subscriber(notifications);
		});
	}

	subscribe(subscriber: Subscriber<IdentifiableNotifications>): Unsubscriber {
		this.subscribers.push(subscriber);

		subscriber(this.copiedNotifications());

		return () => {
			const index = this.subscribers.indexOf(subscriber);
			if (index === -1) {
				return;
			}

			this.subscribers.splice(index, 1);
		};
	}

	private copiedNotifications(): IdentifiableNotifications {
		return [...this.notifications];
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
