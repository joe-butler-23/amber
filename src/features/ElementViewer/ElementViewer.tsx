import ElementContentView from "./ElementContentView";
import FindInPageBar from "./FindInPageBar";
import FolderView from "./FolderView";
import RemoveHighlightModal from "./RemoveHighlightModal";
import LearningAssetView from "./LearningAssetView/LearningAssetView";
import PdfLearningAssetView from "./PdfView/components/PdfLearningAssetView";
import SearchHighlightOverlay from "../../components/Editor/plugins/SearchHighlightPlugin/SearchHighlightOverlay";
import useAppSelector from "../../hooks/useAppSelector";
import { selectCurrentElement } from "../../stores/elements/elementsSelectors";
import { useElementViewerButtons } from "./hooks/useElementViewerButtons";
import { useHighlightCreatedHandler } from "./hooks/useHighlightCreatedHandler";

export default function ElementViewer() {
	const version = useAppSelector(state => state.elements.contentVersion);
	return <CurrentElementViewer key={version} />;
}

function CurrentElementViewer() {
	const currentElement = useAppSelector(selectCurrentElement);
	const elementId = currentElement?.data?.meta?.elementId;
	const {
		buttons,
		pendingHighlightRemoval,
		confirmHighlightRemoval,
		cancelHighlightRemoval,
	} = useElementViewerButtons();
	const handleHighlightCreated = useHighlightCreatedHandler(elementId);

	if (!currentElement || !elementId || currentElement.type === "folder") {
		return <FolderView />;
	}

	const isPdf =
		currentElement.type === "learningAsset" &&
		currentElement.data.type === "pdf";

	if (isPdf) {
		return (
			<PdfLearningAssetView
				key={`pdf-${elementId.id}`}
				learningAssetId={elementId.id}
				readPoint={currentElement.data.readPoint}
			/>
		);
	}

	return (
		<>
			<FindInPageBar />
			<SearchHighlightOverlay />
			<RemoveHighlightModal
				removal={pendingHighlightRemoval}
				onConfirm={confirmHighlightRemoval}
				onClose={cancelHighlightRemoval}
			/>
			{currentElement.type === "learningAsset" && (
				<LearningAssetView
					key={`learningAsset-${elementId.id}`}
					learningAssetId={elementId.id}
					readPoint={currentElement.data.readPoint}
					meta={currentElement.data.meta}
					buttons={buttons}
					onHighlightCreated={handleHighlightCreated}
				/>
			)}
			{(currentElement.type === "card" ||
				currentElement.type === "extract") && (
				<ElementContentView
					key={elementId.id}
					element={currentElement.data}
					buttons={buttons}
					onHighlightCreated={handleHighlightCreated}
				/>
			)}
		</>
	);
}
