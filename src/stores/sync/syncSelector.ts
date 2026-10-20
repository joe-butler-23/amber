import { RootState } from "../store";

export const selectIsSyncing = (state: RootState) => state.sync.isSyncing;
export const selectElementRefreshCount = (state: RootState) =>
	state.elements.contentVersion;

export const selectSyncRevision = (state: RootState) => state.sync.revision;
