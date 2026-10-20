import { notifications } from "@mantine/notifications";
import errorToString from "../utils/errorToString";
import { useEffect } from "react";
import { setCurrentElement } from "../stores/elements/elementsReducer";
import { selectElementTree } from "../stores/elements/elementsSelectors";
import { loadCurrentElementAction } from "../stores/elements/elementsActions";
import useAppDispatch from "./useAppDispatch";
import useAppSelector from "./useAppSelector";
import { useElementParams } from "./useElementParams";
import { ElementId } from "../types/elements/elementId";

export function useCurrentElementSync() {
	const params = useElementParams();
	const tree = useAppSelector(selectElementTree);
	const dispatch = useAppDispatch();

	useEffect(() => {
		if (!params?.type || !params.id) {
			dispatch(setCurrentElement(null));
			return;
		}
		const id = { type: params.type, id: params.id } satisfies ElementId;
		void dispatch(loadCurrentElementAction(id)).catch(error =>
			notifications.show({ message: errorToString(error), color: "red" }),
		);
	}, [params?.type, params?.id, tree, dispatch]);
}
