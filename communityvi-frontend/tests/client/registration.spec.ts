import {Client, RegisteredClient} from '$client/index';
import type {Transport} from '$client/transport';
import {mock} from 'jest-mock-extended';
import type {HelloMessage} from '$client/response';
import {SuccessMessageType} from '$client/response';

describe('Client registrations', () => {
	let id = 0;
	const clients = new Array<number>();
	const mockedTransport = mock<Transport>();
	mockedTransport.performRequest.mockImplementation((_) => {
		const newClient = Promise.resolve(<HelloMessage> {
			type: SuccessMessageType.Hello,
			id: ++id,
			clients: [...clients],
		});
		clients.push(id);

		return newClient;
	});

	it('registers a client', async () => {
		const client = new Client('ws://localhost:8000/ws');

		const registeredClient = await client.registerWithTransport(mockedTransport, 'Max');

		expect(registeredClient).toBeInstanceOf(RegisteredClient);
		expect(registeredClient.id).toBeGreaterThanOrEqual(0);
		expect(registeredClient.name).toBe('Max');
	});

	it('registers multiple clients with individual client IDs', async () => {
		const client = new Client('ws://localhost:8000/ws');

		const registeredClient1 = await client.registerWithTransport(mockedTransport, 'Stephanie');
		const registeredClient2 = await client.registerWithTransport(mockedTransport, 'Johnny 5');

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
