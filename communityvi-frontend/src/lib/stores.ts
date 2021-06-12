import {Writable, writable} from 'svelte/store';
import type {RegisteredClient} from '$lib/client/client';
import {ErrorBag} from '$lib/error_bag';

export const registeredClient: Writable<RegisteredClient | undefined> = writable(undefined);
export const errorBag = new ErrorBag();
