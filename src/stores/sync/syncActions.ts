import { notifications } from "@mantine/notifications";
import { AppDispatch, RootState } from "../store";
import { sync as syncApi } from "../../api/sync/api/syncApi";
import { getElementTree } from "../../api/elements/api/elementsApi";
import { getTrash } from "../../api/trash/api/trashApi";
import { listBibliographicalSources } from "../../api/bibliographicalSources/api/bibliographicalSourcesApi";
import { listAiPrompts } from "../../api/aiIntegration/api/aiApi";
import { flushPendingSaves } from "../../features/ElementViewer/hooks/useAutoSave";
import errorToString from "../../utils/errorToString";
import { setIsSyncing, syncRefreshed } from "./syncReducer";
import {
	selectIsOffline,
	selectIsSignedIn,
	selectUserInformation,
} from "../user/userSelectors";
import { setOnline } from "../user/userReducer";

export interface SyncOptions {
	skipIfKnownOffline?: boolean;
}

export function sync(options?: SyncOptions) {
	return async function (dispatch: AppDispatch, getState: () => RootState) {
		if (
			getState().sync.isSyncing ||
			getState().app.libraryChanging ||
			!selectIsSignedIn(getState()) ||
			!selectUserInformation(getState())?.isEmailVerified
		) {
			return;
		}

		if (options?.skipIfKnownOffline && selectIsOffline(getState())) {
			notifications.show({
				message:
					"You're offline — sync will resume once you're back online.",
				color: "yellow",
			});
			return;
		}

		const generation = getState().app.libraryGeneration;
		const isCurrentLibrary = () =>
			getState().app.libraryGeneration === generation;
		dispatch(setIsSyncing(true));
		try {
			await flushPendingSaves();
			if (!isCurrentLibrary()) return;
			await syncApi();
			if (!isCurrentLibrary()) return;
			dispatch(setOnline());

			const [tree, trash, sources, prompts] = await Promise.allSettled([
				getElementTree(),
				getTrash(),
				listBibliographicalSources(),
				getState().aiPrompts.loaded
					? listAiPrompts()
					: Promise.resolve(undefined),
			]);
			if (!isCurrentLibrary()) return;
			// Publish successful projections and remote-content invalidation together,
			// even if a secondary read failed after the sync committed.
			dispatch(
				syncRefreshed({
					tree: tree.status === "fulfilled" ? tree.value : undefined,
					treeError:
						tree.status === "rejected"
							? errorToString(tree.reason)
							: undefined,
					trash: trash.status === "fulfilled" ? trash.value : undefined,
					trashError:
						trash.status === "rejected"
							? errorToString(trash.reason)
							: undefined,
					sources: sources.status === "fulfilled" ? sources.value : undefined,
					prompts: prompts.status === "fulfilled" ? prompts.value : undefined,
				}),
			);
			for (const result of [tree, trash, sources, prompts]) {
				if (result.status === "rejected")
					notifications.show({
						message: `Synced, but couldn't refresh: ${errorToString(result.reason)}`,
						color: "red",
					});
			}
			notifications.show({ message: "Sync complete", autoClose: 1000 });
		} catch (e) {
			if (!isCurrentLibrary()) return;
			// eslint-disable-next-line no-console
			console.error(e);
			notifications.show({ message: errorToString(e), color: "red" });
		} finally {
			if (isCurrentLibrary()) dispatch(setIsSyncing(false));
		}
	};
}
