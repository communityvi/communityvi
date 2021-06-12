import {Writable, writable} from 'svelte/store';
import type {RegisteredClient} from '$lib/client/client';

export const registeredClient: Writable<RegisteredClient | undefined> = writable(undefined);
