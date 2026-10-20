import { useEffect } from "react";
import { useWindowEvent } from "@mantine/hooks";
import { ELEMENT_CREATED_EVENT } from "../../../api/elements/events/elementCreatedEvent";
import { ELEMENT_DUE_CHANGED_EVENT } from "../../../api/study/events/elementDueChangedEvent";
import { getDueElements } from "../../../api/study/api/studyApi";
import useApi from "../../../hooks/useApi";
import useAppDispatch from "../../../hooks/useAppDispatch";
import useAppSelector from "../../../hooks/useAppSelector";
import { useTauriEvent } from "../../../hooks/useTauriEvent";
import { queueLoaded } from "../../../stores/study/studyReducer";
import { selectStudyStatus } from "../../../stores/study/studySelectors";
import { selectSyncRevision } from "../../../stores/sync/syncSelector";
import { PRIORITY_CHANGED } from "../../../types/events/priorityChangedEvent";
import { STUDY_SESSION_SETTINGS_CHANGED } from "../../../types/events/studySessionSettingsChangedEvent";

// Lets the sidebar preview which elements are due before a session starts,
// without affecting status/counts/etc. Once a session is active, the queue
// is only ever driven by the session engine itself.
export function useDueElementsPreview() {
	const dispatch = useAppDispatch();
	const status = useAppSelector(selectStudyStatus);
	const syncRevision = useAppSelector(selectSyncRevision);
	const libraryGeneration = useAppSelector(state => state.app.libraryGeneration);
	const libraryChanging = useAppSelector(state => state.app.libraryChanging);
	const { callApi } = useApi();

	const isStudying = status === "studying";

	function refresh() {
		void callApi(() =>
			dispatch(async (dispatch, getState) => {
				if (getState().app.libraryChanging) return;
				const generation = getState().app.libraryGeneration;
				const queue = await getDueElements();
				if (getState().app.libraryGeneration === generation)
					dispatch(queueLoaded(queue));
			}),
		);
	}

	useEffect(() => {
		if (isStudying) return;
		refresh();
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [
		isStudying,
		syncRevision,
		libraryGeneration,
		libraryChanging,
		callApi,
		dispatch,
	]);

	useTauriEvent(ELEMENT_CREATED_EVENT, () => {
		if (isStudying) return;
		refresh();
	});

	useWindowEvent(PRIORITY_CHANGED, () => {
		if (isStudying) return;
		refresh();
	});

	useTauriEvent(ELEMENT_DUE_CHANGED_EVENT, () => {
		if (isStudying) return;
		refresh();
	});

	useWindowEvent(STUDY_SESSION_SETTINGS_CHANGED, () => {
		if (isStudying) return;
		refresh();
	});
}
