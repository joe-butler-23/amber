import { useEffect } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
	$getRoot,
	$getSelection,
	$isRangeSelection,
	COMMAND_PRIORITY_EDITOR,
} from "lexical";
import { $wrapSelectionInMarkNode } from "@lexical/mark";
import { $generateJSONFromSelectedNodes } from "@lexical/clipboard";
import { type SerializedLexicalNodeTree } from "../../lexicalJsonConversion";
import { $createHighlightNode, HighlightNode } from "./HighlightNode";
import {
	CREATE_HIGHLIGHT_COMMAND,
	HIGHLIGHT_COLORS,
	HighlightCreatedPayload,
} from "./highlightCommands";
import { $trimSelectionWhitespace } from "./trimSelectionWhitespace";

interface Props {
	onHighlightCreated?: (payload: HighlightCreatedPayload) => void;
}

export function HighlightPlugin({ onHighlightCreated }: Props) {
	const [editor] = useLexicalComposerContext();

	useEffect(() => {
		if (!editor.hasNodes([HighlightNode])) {
			throw new Error(
				"HighlightPlugin: HighlightNode not registered in editor",
			);
		}
		return editor.registerCommand(
			CREATE_HIGHLIGHT_COMMAND,
			action => {
				const color = HIGHLIGHT_COLORS[action];
				const selection = $getSelection();
				if (!$isRangeSelection(selection) || selection.isCollapsed()) {
					return false;
				}
				if (!selection.getTextContent().trim()) return false;
				$trimSelectionWhitespace(selection);
				const selectionText = selection.getTextContent();

				const { nodes: selectionNodes } =
					$generateJSONFromSelectedNodes<SerializedLexicalNodeTree>(
						editor,
						selection,
					);
				// Whichever point comes later in document order is the end of
				// the extracted range, regardless of which way the user dragged.
				const endPoint = selection.isBackward()
					? selection.anchor
					: selection.focus;
				const endBlockIndex = endPoint
					.getNode()
					.getTopLevelElementOrThrow()
					.getIndexWithinParent();

				const id = crypto.randomUUID();
				$wrapSelectionInMarkNode(
					selection,
					selection.isBackward(),
					id,
					ids => $createHighlightNode(ids, color),
				);
				// Read live after wrapping; a committed editor state would miss the mark.
				const cloze =
					action === "cloze"
						? {
							nodes: $generateJSONFromSelectedNodes<SerializedLexicalNodeTree>(
								editor,
								null,
							).nodes,
							text: $getRoot().getTextContent(),
						}
						: undefined;
				onHighlightCreated?.({
					id,
					selectionNodes,
					cloze,
					selectionText,
					endBlockIndex,
				});
				return true;
			},
			COMMAND_PRIORITY_EDITOR,
		);
	}, [editor, onHighlightCreated]);

	return null;
}
