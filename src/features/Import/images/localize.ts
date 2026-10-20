import { fetchImage } from "../../../api/import/api/importApi";
import { createAssets } from "../../../api/assets/api/assetsApi";
import { compressDataUri, estimateDataUriBytes } from "./compressImage";
import { SVG_MIME, toBase64SvgDataUri } from "./svg";

export type LocalizedImage =
	{ ok: true; src: string } | { ok: false; originalUrl: string };

type PreparedImage =
	{ ok: true; dataUri: string } | { ok: false; originalUrl: string };

const MAX_DATA_URI_BYTES = 2 * 1024 * 1024;
const IMAGE_BATCH_SIZE = 4;

/** The only module that knows how imported images are stored: as assets, referenced by `amber-asset:<id>`. */
export async function localizeImage(
	absoluteUrl: string,
	referer: string | null,
): Promise<LocalizedImage> {
	const localized = await localizeImages([absoluteUrl], referer);
	return localized.get(absoluteUrl)!;
}

/** Bounds concurrent download/compression work and releases image data after each stored batch. */
export async function localizeImages(
	absoluteUrls: string[],
	referer: string | null,
): Promise<Map<string, LocalizedImage>> {
	const result = new Map<string, LocalizedImage>();
	for (
		let offset = 0;
		offset < absoluteUrls.length;
		offset += IMAGE_BATCH_SIZE
	) {
		const batch = absoluteUrls.slice(offset, offset + IMAGE_BATCH_SIZE);
		const prepared = await Promise.all(
			batch.map(url => prepareImage(url, referer)),
		);
		const srcs = await storeImages(
			prepared.flatMap(image => (image.ok ? [image.dataUri] : [])),
		);
		let successIndex = 0;
		prepared.forEach((image, index) => {
			const url = batch[index];
			const src = image.ok ? srcs[successIndex++] : null;
			result.set(
				url,
				src ? { ok: true, src } : { ok: false, originalUrl: url },
			);
		});
	}
	return result;
}

async function prepareImage(
	absoluteUrl: string,
	referer: string | null,
): Promise<PreparedImage> {
	if (absoluteUrl.startsWith("data:")) {
		if (absoluteUrl.startsWith(`data:${SVG_MIME}`)) {
			return prepareSvg(absoluteUrl, absoluteUrl);
		}
		return compressImage(absoluteUrl, absoluteUrl);
	}

	let fetched: Awaited<ReturnType<typeof fetchImage>>;
	try {
		fetched = await fetchImage(absoluteUrl, referer);
	} catch {
		return { ok: false, originalUrl: absoluteUrl };
	}

	const dataUri = `data:${fetched.mime};base64,${fetched.bytesBase64}`;
	if (fetched.mime === SVG_MIME) return prepareSvg(dataUri, absoluteUrl);
	return compressImage(dataUri, absoluteUrl);
}

// Vector images stay as they are: there's nothing to gain from re-encoding them as rasters.
function prepareSvg(dataUri: string, originalUrl: string): PreparedImage {
	const base64 = toBase64SvgDataUri(dataUri);
	if (base64 === null || estimateDataUriBytes(base64) > MAX_DATA_URI_BYTES) {
		return { ok: false, originalUrl };
	}
	return { ok: true, dataUri: base64 };
}

async function compressImage(
	dataUri: string,
	originalUrl: string,
): Promise<PreparedImage> {
	const compressed = await compressDataUri(dataUri, MAX_DATA_URI_BYTES);
	if (compressed.ok) return { ok: true, dataUri: compressed.src };

	if (compressed.reason === "too-large") {
		return { ok: false, originalUrl };
	}

	if (estimateDataUriBytes(dataUri) > MAX_DATA_URI_BYTES) {
		return { ok: false, originalUrl };
	}

	// The browser couldn't decode it, so the backend likely can't either.
	return { ok: true, dataUri };
}

// A failed store marks the images broken, like a failed fetch, instead of aborting the whole import.
async function storeImages(dataUris: string[]): Promise<(string | null)[]> {
	if (dataUris.length === 0) return [];
	try {
		const assets = await createAssets({ dataUris });
		return assets.map(asset => asset?.src ?? null);
	} catch {
		return dataUris.map(() => null);
	}
}
