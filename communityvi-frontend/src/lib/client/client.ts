import type {Transport} from '$lib/client/transport';
import {VersionedMedium} from '$lib/client/model';
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
		await this.restClient.registerNewUser(name);
		const token = await this.restClient.login(name);
		const connection = await this.transport.connect(token);
		const currentMedium = await this.restClient.defaultMedium(token);

		const referenceTimeSynchronizer = await ReferenceTimeSynchronizer.createInitializedWithRESTClient(
			this.restClient,
		);
		const versionedMedium = VersionedMedium.fromVersionedMediumResponseAndReferenceTimeOffset(
			currentMedium,
			referenceTimeSynchronizer.offset,
		);

		return new RegisteredClient(
			name,
			referenceTimeSynchronizer,
			versionedMedium,
			this.restClient,
			connection,
			disconnectCallback,
		);
	}
}
