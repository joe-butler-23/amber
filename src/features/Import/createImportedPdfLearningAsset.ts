import { createLearningAssetAction } from "../../stores/elements/elementsActions";
import { ImportContext } from "./importContext";

/** Creates the learning asset and returns its id; the caller decides whether to open it. */
export async function createImportedPdfLearningAsset(
	ctx: ImportContext,
	name: string,
	pdfBytesBase64: string,
	pdfPageCount: number,
	bibliographicalSourceId?: string | null,
): Promise<string> {
	const id = crypto.randomUUID();
	const created = await ctx.dispatch(
		createLearningAssetAction({
			id,
			meta: {
				name,
				parent: ctx.parent,
				origin: { type: "custom", bibliographicalSourceId },
			},
			type: "pdf",
			pdfBytesBase64,
			pdfPageCount,
			splits: [],
			initialPriorityPosition: ctx.priorityPosition,
		}),
	);
	if (!created) throw new Error("The document could not be saved.");
	return id;
}
