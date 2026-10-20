import useAutoSave from "../../hooks/useAutoSave";
import { useEffect } from "react";
import { notifications } from "@mantine/notifications";
import {
	AnnotationTransferItem,
	useAnnotationCapability,
} from "@embedpdf/plugin-annotation/react";
import {
	getPdfHighlights,
	updatePdfHighlights,
} from "../../../../api/elements/api/elementsApi";
import errorToString from "../../../../utils/errorToString";
import { isPdfHighlightAnnotation } from "../components/pdfHighlightAnnotations";

function showHighlightError(message: string, error: unknown) {
	// eslint-disable-next-line no-console
	console.error(error);
	notifications.show({
		message: `${message} ${errorToString(error)}`,
		color: "red",
	});
}

/** Loads a PDF learning asset's persisted highlight annotations into
 * `@embedpdf/plugin-annotation` once its document is ready, and writes them
 * back (as `AnnotationTransferItem[]` JSON) whenever they change. The
 * backend's `highlights` column is an opaque JSON blob it never parses, so
 * the annotation plugin's own transfer format is what gets stored.
 *
 * `exportAnnotations()` returns *every* annotation the plugin knows about,
 * including the PDF's own native ones (links, etc.) that `getAllAnnotations`
 * loads on every open — those must never be persisted here, both because
 * they're not ours to own and because a large PDF can have thousands of
 * them, ballooning the blob and making every future import crawl. */
export function usePdfAnnotationsPersistence(
	documentId: string | null | undefined,
	learningAssetId: string,
) {
	const { provides: annotation } = useAnnotationCapability();
	const onContentUpdate = useAutoSave(highlightsJson =>
		updatePdfHighlights({ learningAssetId, highlightsJson }),
	);

	useEffect(() => {
		if (!annotation || !documentId) return;
		const scope = annotation.forDocument(documentId);
		let cancelled = false;
		const loaded = getPdfHighlights(learningAssetId).then(
			({ highlightsJson }) => {
				if (cancelled)
					throw new Error("PDF closed before its highlights loaded");
				scope.importAnnotations(
					JSON.parse(highlightsJson) as AnnotationTransferItem[],
				);
			},
		);
		void loaded.catch(error =>
			showHighlightError("Could not load this PDF's highlights.", error),
		);
		const unsubscribe = scope.onAnnotationEvent(event => {
			if (event.type === "loaded") return;
			onContentUpdate(async () => {
				await loaded;
				const exported = await scope.exportAnnotations().toPromise();
				return JSON.stringify(
					exported.filter(item =>
						isPdfHighlightAnnotation(item.annotation),
					),
				);
			});
		});
		return () => {
			cancelled = true;
			unsubscribe();
		};
	}, [annotation, documentId, learningAssetId, onContentUpdate]);
}
