import {promiseWithTimout} from '$lib/client/promises';
import {Medium} from '$lib/client/model';

export default class MetadataLoader {
	private readonly player: HTMLMediaElement;

	private pendingMetadataLoad?: PendingMetadataLoad;

	constructor(player: HTMLMediaElement) {
		this.player = player;

		this.player.preload = 'metadata';
		this.player.onloadedmetadata = () => this.onLoadedMetadata();
		this.player.onerror = () => this.onError();
	}

	async selectedMediumFromFile(file: File): Promise<SelectedMedium> {
		if (this.pendingMetadataLoad !== undefined) {
			throw Error('Already loading');
		}

		const loadingPromise = new Promise<SelectedMedium>((resolve, reject) => {
			this.pendingMetadataLoad = new PendingMetadataLoad(file.name, resolve, reject);

			this.player.src = URL.createObjectURL(file);
		});

		return promiseWithTimout(loadingPromise, 60_000, () => this.reset());
	}

	private onLoadedMetadata() {
		if (this.pendingMetadataLoad === undefined) {
			console.error('Loaded metadata, but there was no load pending.');
			return;
		}

		const pendingMetadataLoad = this.pendingMetadataLoad;
		const duration = this.player.duration;
		this.reset();

		const durationInMilliseconds = Math.round(duration * 1000);
		const selectedMedium = new SelectedMedium(pendingMetadataLoad.name, durationInMilliseconds);
		pendingMetadataLoad.resolve(selectedMedium);
	}

	private onError() {
		if (this.pendingMetadataLoad === undefined) {
			return;
		}

		const pendingMetadataLoad = this.pendingMetadataLoad;
		this.reset();

		pendingMetadataLoad.reject(new PlayerLoadError(this.player.error));
	}

	private reset() {
		// see https://html.spec.whatwg.org/multipage/media.html#best-practices-for-authors-using-media-elements
		this.player.removeAttribute('src');
		this.player.load();

		this.pendingMetadataLoad = undefined;
	}
}

class PendingMetadataLoad {
	name: string;

	resolve: (selectedMedium: SelectedMedium) => void;
	reject: (error: Error) => void;

	constructor(name: string, resolve: (selectedMedium: SelectedMedium) => void, reject: (error: Error) => void) {
		this.name = name;

		this.resolve = resolve;
		this.reject = reject;
	}
}

export class SelectedMedium {
	readonly name: string;
	readonly lengthInMilliseconds: number;

	constructor(name: string, lengthInMilliseconds: number) {
		this.name = name;
		this.lengthInMilliseconds = lengthInMilliseconds;
	}

	isMeaningfullyDifferentTo(medium: Medium): boolean {
		const ownLengthInSecondsTruncated = Math.trunc(this.lengthInMilliseconds / 1000);
		const mediumLengthInSecondsTruncated = Math.trunc(medium.lengthInMilliseconds / 1000);

		return (
			this.name.trim() !== medium.name.trim() || ownLengthInSecondsTruncated !== mediumLengthInSecondsTruncated
		);
	}
}

class PlayerLoadError extends Error {
	readonly code?: number;

	constructor(mediaError: MediaError | null) {
		super(mediaError ? `Error while loading file: ${mediaError.message}` : 'Unknown error while loading file.');

		this.name = PlayerLoadError.name;
		this.code = mediaError?.code;
	}
}
