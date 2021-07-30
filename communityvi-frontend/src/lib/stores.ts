import {Writable, writable} from 'svelte/store';
import type RegisteredClient from '$lib/client/registered_client';
import {NotificationStore} from '$lib/components/notification/notification_store';

export const registeredClient: Writable<RegisteredClient | undefined> = writable(undefined);
export const notifications = new NotificationStore();
export const videoUrl: Writable<string | undefined> = writable(undefined);
