import { RootState } from "../store";

export const selectStartedInitialStateLoading = (state: RootState) =>
	state.app.startedInitialStateLoading;

export const selectAiPromptLibraryNewPromptRequest = (state: RootState) =>
	state.app.aiPromptLibraryNewPromptRequest;

export const selectAuthModalInitialTab = (state: RootState) =>
	state.app.authModalInitialTab;

export const selectIsVirtualKeyboardSuppressed = (state: RootState) =>
	state.app.virtualKeyboardSuppressed;
