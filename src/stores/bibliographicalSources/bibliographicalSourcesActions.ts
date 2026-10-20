import {
	assignBibliographicalSource,
	createBibliographicalSource,
	deleteBibliographicalSource,
	listBibliographicalSources,
	updateBibliographicalSource,
} from "../../api/bibliographicalSources/api/bibliographicalSourcesApi";
import {
	BibliographicalSourceRequestDto,
	BibliographicalSourceResponseDto,
} from "../../api/bibliographicalSources/dto/bibliographicalSourceDto";
import { ElementId } from "../../types/elements/elementId";
import { setCurrentElementMeta } from "../elements/elementsReducer";
import { AppDispatch, RootState } from "../store";
import {
	removeBibliographicalSource,
	setBibliographicalSources,
	setBibliographicalSourcesLoading,
	upsertBibliographicalSource,
} from "./bibliographicalSourcesReducer";
import { selectBibliographicalSourcesLoaded } from "./bibliographicalSourcesSelectors";

export function loadBibliographicalSourcesAction() {
	return async (dispatch: AppDispatch, getState: () => RootState) => {
		if (
			getState().app.libraryChanging ||
			selectBibliographicalSourcesLoaded(getState())
		)
			return;
		const generation = getState().app.libraryGeneration;
		dispatch(setBibliographicalSourcesLoading());
		try {
			const sources = await listBibliographicalSources();
			if (getState().app.libraryGeneration === generation)
				dispatch(setBibliographicalSources(sources));
		} finally {
			if (getState().app.libraryGeneration === generation)
				dispatch(setBibliographicalSourcesLoading(false));
		}
	};
}

export function createBibliographicalSourceAction(
	dto: BibliographicalSourceRequestDto,
) {
	return async (
		dispatch: AppDispatch,
	): Promise<BibliographicalSourceResponseDto> => {
		const created = await createBibliographicalSource(dto);
		dispatch(upsertBibliographicalSource(created));
		return created;
	};
}

export function updateBibliographicalSourceAction(
	id: string,
	dto: BibliographicalSourceRequestDto,
) {
	return async (dispatch: AppDispatch) => {
		const updated = await updateBibliographicalSource(id, dto);
		dispatch(upsertBibliographicalSource(updated));
	};
}

export function deleteBibliographicalSourceAction(id: string) {
	return async (dispatch: AppDispatch) => {
		await deleteBibliographicalSource(id);
		dispatch(removeBibliographicalSource(id));
	};
}

export function assignBibliographicalSourceAction(
	elementId: ElementId,
	bibliographicalSourceId: string | null,
) {
	return async (dispatch: AppDispatch) => {
		await assignBibliographicalSource(elementId, bibliographicalSourceId);
		dispatch(setCurrentElementMeta({ bibliographicalSourceId }));
	};
}
