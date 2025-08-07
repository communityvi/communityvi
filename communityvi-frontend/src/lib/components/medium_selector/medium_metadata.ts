import {Medium} from '$lib/client/model';

export class MediumMetadata {
	readonly name: string;
	readonly lengthInMilliseconds: number;

	constructor(name: string, lengthInMilliseconds: number) {
		this.name = name;
		this.lengthInMilliseconds = lengthInMilliseconds;
	}

	isMeaningfullyDifferentTo(medium: Medium): boolean {
		// NOTE: Different browsers tend to subtly disagree in how long the same given video file is.
		const delta = Math.abs(this.lengthInMilliseconds - medium.lengthInMilliseconds);
		const lengthDifferenceThreshold = 500;

		return this.name.trim() !== medium.name.trim() || delta >= lengthDifferenceThreshold;
	}

	static async fromFile(file: File): Promise<MediumMetadata> {
		const videoElement = document.createElement('video');
		videoElement.preload = 'metadata';

		const url = URL.createObjectURL(file);

		const durationSeconds = await new Promise<number>((resolve, reject) => {
			videoElement.onloadedmetadata = () => resolve(videoElement.duration);
			videoElement.onerror = () => reject(new PlayerLoadError(videoElement.error));
			videoElement.src = url;
		});

		return new MediumMetadata(file.name, Math.round(durationSeconds * 1000));
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
