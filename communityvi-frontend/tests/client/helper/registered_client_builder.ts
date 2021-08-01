import {Peer, VersionedMedium} from '$lib/client/model';
import RegisteredClient, {DisconnectCallback} from '$lib/client/registered_client';
import {mock} from 'jest-mock-extended';
import type ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';
import type {Connection} from '$lib/client/connection';
import Faker from 'faker';

export class RegisteredClientBuilder {
	private storedID = Faker.datatype.number({min: 0});
	private storedName = Faker.internet.userName();
	private storedReferenceTimeSynchronizer: ReferenceTimeSynchronizer = mock<ReferenceTimeSynchronizer>();
	private storedVersionedMedium = new VersionedMedium(Faker.datatype.number({min: 0}));
	private storedPeers = new Array<Peer>();
	private storedConnection: Connection = mock<Connection>();
	private storedDisconnectCallback: DisconnectCallback = mock<DisconnectCallback>();

	static default(): RegisteredClientBuilder {
		return new RegisteredClientBuilder();
	}

	// eslint-disable-next-line @typescript-eslint/no-empty-function
	private constructor() {}

	id(id: number): RegisteredClientBuilder {
		this.storedID = id;

		return this;
	}

	name(name: string): RegisteredClientBuilder {
		this.storedName = name;

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
			this.storedConnection,
			this.storedDisconnectCallback,
		);
	}
}
