import {Client, RegisteredClient} from '$client/client';
import TestTransport from './helper/test_transport';

describe('Client registrations', () => {
	const transport = new TestTransport();
	const client = new Client(transport);

	it('registers a client', async () => {
		const registeredClient = await client.register('Max');

		expect(registeredClient).toBeInstanceOf(RegisteredClient);
		expect(registeredClient.id).toBeGreaterThanOrEqual(0);
		expect(registeredClient.name).toBe('Max');
	});

	it('registers multiple clients with individual client IDs', async () => {
		const registeredClient1 = await client.register('Stephanie');
		const registeredClient2 = await client.register('Johnny 5');

		expect(registeredClient1).toBeInstanceOf(RegisteredClient);
		expect(registeredClient1.id).toBeGreaterThanOrEqual(0);
		expect(registeredClient1.name).toBe('Stephanie');

		expect(registeredClient2).toBeInstanceOf(RegisteredClient);
		expect(registeredClient2.id).toBeGreaterThanOrEqual(0);
		expect(registeredClient2.name).toBe('Johnny 5');

		expect(registeredClient2.id).toBeGreaterThan(registeredClient1.id);
	});
});

export {};
