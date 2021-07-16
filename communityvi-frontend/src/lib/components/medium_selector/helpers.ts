import type {Medium} from '$lib/client/model';

export function formatMediumLength(medium?: Medium, locale = 'en-US'): string | undefined {
	if (medium === undefined) {
		return undefined;
	}

	let formattedLength = '';
	let remainingSeconds = medium.lengthInMilliseconds / 1000;

	const hours = Math.floor(remainingSeconds / (60 * 60));
	if (hours > 0) {
		remainingSeconds %= 60 * 60;
		formattedLength += `${hours}h `;
	}

	const minutes = Math.floor(remainingSeconds / 60);
	if (minutes > 0) {
		remainingSeconds %= 60;
		formattedLength += `${minutes}min `;
	}

	const seconds = remainingSeconds;
	if (seconds > 0) {
		formattedLength += `${new Intl.NumberFormat(locale, {maximumFractionDigits: 2}).format(seconds)}s`;
	}

	return formattedLength.trimEnd();
}
