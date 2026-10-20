import { PayloadAction, createSlice } from "@reduxjs/toolkit";
import { beginLibraryChange } from "../app/appReducer";
import { NodeDto } from "../../api/elements/dto/nodeDto";
import { TrashedElementDto } from "../../api/trash/dto/trashedElementDto";
import { BibliographicalSourceResponseDto } from "../../api/bibliographicalSources/dto/bibliographicalSourceDto";
import AiPromptDto from "../../api/aiIntegration/dto/aiPromptDto";

interface SyncState {
	isSyncing: boolean;
	revision: number;
}

export interface SyncedProjections {
	tree?: NodeDto[];
	treeError?: string;
	trash?: TrashedElementDto[];
	trashError?: string;
	sources?: BibliographicalSourceResponseDto[];
	prompts?: AiPromptDto[];
}

const initialState: SyncState = {
	isSyncing: false,
	revision: 0,
};

const syncSlice = createSlice({
	name: "sync",
	initialState,
	reducers: {
		setIsSyncing: (state, payload: PayloadAction<boolean>) => {
			state.isSyncing = payload.payload;
		},
		syncRefreshed: (state, _action: PayloadAction<SyncedProjections>) => {
			state.revision++;
		},
	},
	extraReducers: builder => {
		builder.addCase(beginLibraryChange, state => {
			state.isSyncing = false;
		});
	},
});

export default syncSlice.reducer;

export const { setIsSyncing, syncRefreshed } = syncSlice.actions;
