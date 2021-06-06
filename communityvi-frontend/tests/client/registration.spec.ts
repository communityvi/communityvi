import {Client, RegisteredClient} from '$client/index';

describe('Client registrations', () => {
	it('registers a client', async () => {
		const client = new Client('ws://localhost:8000/ws');

		const registeredClient = await client.register('Max');

		expect(registeredClient).toBeInstanceOf(RegisteredClient);
		expect(registeredClient.id).toBeGreaterThanOrEqual(0);
		expect(registeredClient.name).toBe('Max');
	});

	it('registers multiple clients with individual client IDs', async () => {
		const client = new Client('ws://localhost:8000/ws');

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
