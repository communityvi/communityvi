import type {HelloMessage} from '$lib/client/response';
import {RegisterRequest} from '$lib/client/request';
import type {Transport} from '$lib/client/transport';
import {Peer, VersionedMedium} from '$lib/client/model';
import ReferenceTimeSynchronizer from '$lib/client/reference_time_synchronizer';
import RegisteredClient, {DisconnectCallback} from '$lib/client/registered_client';
import {RESTClient} from '$lib/client/RESTClient';

export default class Client {
	readonly transport: Transport;
	readonly restClient: RESTClient;

	constructor(transport: Transport, restClient: RESTClient) {
		this.transport = transport;
		this.restClient = restClient;
	}

	async register(name: string, disconnectCallback: DisconnectCallback): Promise<RegisteredClient> {
		const connection = await this.transport.connect();

		const response = (await connection.performRequest(new RegisterRequest(name))).response as HelloMessage;
		const peers = response.clients.map(Peer.fromClientResponse);

		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithRESTClient(
			this.restClient,
		);
		const versionedMedium = VersionedMedium.fromVersionedMediumResponseAndReferenceTimeOffset(
			response.current_medium,
			referenceTimeSynchronizer.offset,
		);

		return new RegisteredClient(
			response.id,
			name,
			referenceTimeSynchronizer,
			versionedMedium,
			peers,
			this.restClient,
			connection,
			disconnectCallback,
		);
	}
}
