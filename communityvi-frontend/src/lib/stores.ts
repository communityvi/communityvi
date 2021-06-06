import {Writable, writable} from 'svelte/store';
import type {RegisteredClient} from '$lib/client/client';

export const registeredClientStore: Writable<RegisteredClient | undefined> = writable(undefined);
