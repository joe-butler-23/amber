import { notifications } from "@mantine/notifications";
import errorToString from "../utils/errorToString";
import { useEffect, useRef } from "react";
import { setCurrentElement } from "../stores/elements/elementsReducer";
import { selectElementTree } from "../stores/elements/elementsSelectors";
import { loadCurrentElementAction } from "../stores/elements/elementsActions";
import useAppDispatch from "./useAppDispatch";
import useAppSelector from "./useAppSelector";
import { useElementParams } from "./useElementParams";
import { ElementId } from "../types/elements/elementId";
import { selectSyncRevision } from "../stores/sync/syncSelector";

export function useCurrentElementSync() {
	const params = useElementParams();
	const tree = useAppSelector(selectElementTree);
	const dispatch = useAppDispatch();
	const syncRevision = useAppSelector(selectSyncRevision);
	const libraryGeneration = useAppSelector(state => state.app.libraryGeneration);
	const libraryChanging = useAppSelector(state => state.app.libraryChanging);
	const previousSyncRevision = useRef(syncRevision);

	useEffect(() => {
		if (libraryChanging) return;
		const refreshContent = previousSyncRevision.current !== syncRevision;
		previousSyncRevision.current = syncRevision;
		if (!params?.type || !params.id) {
			dispatch(setCurrentElement(null));
			return;
		}
		const id = { type: params.type, id: params.id } satisfies ElementId;
		void dispatch(loadCurrentElementAction(id, refreshContent)).catch(error =>
			notifications.show({ message: errorToString(error), color: "red" }),
		);
	}, [
		params?.type,
		params?.id,
		tree,
		syncRevision,
		libraryGeneration,
		libraryChanging,
		dispatch,
	]);
}
