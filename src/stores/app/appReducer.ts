import { PayloadAction, createSlice } from "@reduxjs/toolkit";
import { loadVirtualKeyboardSuppressed } from "./virtualKeyboardStorage";

export type AuthModalTab = "sign-in" | "sign-up";
export type ModalName =
	| "import"
	| "studyProfile"
	| "settings"
	| "priority"
	| "dueDate"
	| "shortcuts"
	| "aiPromptLibrary"
	| "studySessionSettings"
	| "auth"
	| "verifyEmail"
	| "manageAccount";

type SimpleModalName = Exclude<ModalName, "auth" | "aiPromptLibrary">;

export interface AppState {
	startedInitialStateLoading: boolean;
	openModals: Partial<Record<ModalName, true>>;
	/** Bumped on each request to start a blank new prompt in the library, even while it's open. */
	aiPromptLibraryNewPromptRequest: number;
	authModalInitialTab: AuthModalTab;
	/** When true the editor never asks for the on-screen keyboard, so a touch
	 * user can select text and make extracts without it covering the page. */
	virtualKeyboardSuppressed: boolean;
}

const initialState: AppState = {
	startedInitialStateLoading: false,
	openModals: {},
	aiPromptLibraryNewPromptRequest: 0,
	authModalInitialTab: "sign-in",
	virtualKeyboardSuppressed: loadVirtualKeyboardSuppressed(),
};

const appSlice = createSlice({
	name: "app",
	initialState,
	reducers: {
		markStartLoadingOfInitialState: state => {
			state.startedInitialStateLoading = true;
		},
		openModal: (state, action: PayloadAction<SimpleModalName>) => {
			state.openModals[action.payload] = true;
		},
		closeModal: (state, action: PayloadAction<ModalName>) => {
			delete state.openModals[action.payload];
		},
		openAiPromptLibraryModal: (
			state,
			action: PayloadAction<{ newPrompt?: boolean } | undefined>,
		) => {
			state.openModals.aiPromptLibrary = true;
			if (action.payload?.newPrompt)
				state.aiPromptLibraryNewPromptRequest += 1;
		},
		openAuthModal: (
			state,
			action: PayloadAction<AuthModalTab | undefined>,
		) => {
			state.openModals.auth = true;
			state.authModalInitialTab = action.payload ?? "sign-in";
		},
		setVirtualKeyboardSuppressed: (
			state,
			action: PayloadAction<boolean>,
		) => {
			state.virtualKeyboardSuppressed = action.payload;
		},
	},
});

export default appSlice.reducer;

export const {
	markStartLoadingOfInitialState,
	openModal,
	closeModal,
	openAiPromptLibraryModal,
	openAuthModal,
	setVirtualKeyboardSuppressed,
} = appSlice.actions;
