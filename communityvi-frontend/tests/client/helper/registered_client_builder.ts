import {Peer, VersionedMedium} from '$lib/client/model';
import RegisteredClient, {DisconnectCallback} from '$lib/client/registered_client';
import {mock} from 'jest-mock-extended';
import type ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';
import type {Connection} from '$lib/client/connection';
import {faker} from '@faker-js/faker';
import {RESTClient} from '$lib/client/RESTClient';

export class RegisteredClientBuilder {
	private storedID = faker.number.int({min: 0});
	private storedName = faker.internet.userName();
	private storedReferenceTimeOffset = 0;
	private storedReferenceTimeSynchronizer: ReferenceTimeSynchronizer;
	private storedVersionedMedium = new VersionedMedium(faker.number.int({min: 0}));
	private storedPeers = new Array<Peer>();
	private storedRestClient: RESTClient = mock<RESTClient>();
	private storedConnection: Connection = mock<Connection>();
	private storedDisconnectCallback: DisconnectCallback = jest.fn();

	static default(): RegisteredClientBuilder {
		return new RegisteredClientBuilder();
	}

	private constructor() {
		// We initialize the mock in the constructor to guarantee a non-NaN return value, which is what we want most times.
		const referenceTimeSynchronizerMock = mock<ReferenceTimeSynchronizer>();
		referenceTimeSynchronizerMock.calculateServerTimeFromLocalTime.mockImplementation(
			localTimeInMilliseconds => localTimeInMilliseconds + this.storedReferenceTimeOffset,
		);

		this.storedReferenceTimeSynchronizer = referenceTimeSynchronizerMock;
	}

	id(id: number): RegisteredClientBuilder {
		this.storedID = id;

		return this;
	}

	name(name: string): RegisteredClientBuilder {
		this.storedName = name;

		return this;
	}

	referenceTimeOffset(offset: number): RegisteredClientBuilder {
		this.storedReferenceTimeOffset = offset;

		return this;
	}

	referenceTimeSynchronizer(referenceTimeSynchronizer: ReferenceTimeSynchronizer): RegisteredClientBuilder {
		this.storedReferenceTimeSynchronizer = referenceTimeSynchronizer;

		return this;
	}

	versionedMedium(versionedMedium: VersionedMedium): RegisteredClientBuilder {
		this.storedVersionedMedium = versionedMedium;

		return this;
	}

	peers(peers: Array<Peer>): RegisteredClientBuilder {
		this.storedPeers = peers;

		return this;
	}

	peer(peer: Peer): RegisteredClientBuilder {
		this.storedPeers.push(peer);

		return this;
	}

	restClient(restClient: RESTClient): RegisteredClientBuilder {
		this.storedRestClient = restClient;

		return this;
	}

	connection(connection: Connection): RegisteredClientBuilder {
		this.storedConnection = connection;

		return this;
	}

	disconnectCallback(disconnectCallback: DisconnectCallback): RegisteredClientBuilder {
		this.storedDisconnectCallback = disconnectCallback;

		return this;
	}

	build(): RegisteredClient {
		return new RegisteredClient(
			this.storedID,
			this.storedName,
			this.storedReferenceTimeSynchronizer,
			this.storedVersionedMedium,
			this.storedPeers,
			this.storedRestClient,
			this.storedConnection,
			this.storedDisconnectCallback,
		);
	}
}
