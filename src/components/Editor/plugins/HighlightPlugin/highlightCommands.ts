import { createCommand } from "lexical";
import { type SerializedLexicalNodeTree } from "../../lexicalJsonConversion";

export const HIGHLIGHT_COLORS = { extract: "yellow", cloze: "blue" } as const;

export const CREATE_HIGHLIGHT_COMMAND =
	createCommand<keyof typeof HIGHLIGHT_COLORS>("CREATE_HIGHLIGHT_COMMAND");

export interface HighlightCreatedPayload {
	id: string;
	/** Serialized nodes of just the selected range, before wrapping. */
	selectionNodes: SerializedLexicalNodeTree[];
	selectionText: string;
	/** Only clozes need the whole document, after wrapping the selection. */
	cloze?: { nodes: SerializedLexicalNodeTree[]; text: string };
	/** Index of the top-level block containing the end of the selection. */
	endBlockIndex: number;
}
