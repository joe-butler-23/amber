import { flushPendingSaves } from "../../features/ElementViewer/hooks/useAutoSave";
import { NavigateFunction } from "react-router";
import { loadSettings, saveSettings } from "../settings/settingsActions";
import { buildUpdateSettingsRequest } from "../../api/settings/dto/updateSettingsRequestDto";
import { AppDispatch, RootState } from "../store";
import { sync } from "../sync/syncActions";
import { loadUserState } from "../user/userActions";
import { UserInformationDto } from "../../api/backend/dto/userInformationDto";
import { setUserInformation } from "../user/userReducer";
import { saveCachedUserInformation } from "../user/userInformationCache";
import {
	selectIsVirtualKeyboardSuppressed,
	selectStartedInitialStateLoading,
} from "./appSelectors";
import {
	beginLibraryChange,
	libraryChanged,
	finishLibraryChange,
	markStartLoadingOfInitialState,
	setVirtualKeyboardSuppressed,
} from "./appReducer";
import { saveVirtualKeyboardSuppressed } from "./virtualKeyboardStorage";
import { loadElementTree } from "../elements/elementsActions";
import { setCurrentElement } from "../elements/elementsReducer";

export function initialLoadApplicationState() {
	return async function (
		dispatch: AppDispatch,
		getState: () => RootState,
	): Promise<void> {
		if (selectStartedInitialStateLoading(getState())) return;
		dispatch(markStartLoadingOfInitialState());
		await loadAppState(dispatch);
	};
}

/** Owns native library changes and reloading their views, including failed attempts. */
export function reloadApplicationState(
	navigate: NavigateFunction,
	change?: (dispatch: AppDispatch) => Promise<UserInformationDto | void>,
) {
	return async function (dispatch: AppDispatch, getState: () => RootState) {
		if (getState().app.libraryChanging)
			throw new Error("A library change is already in progress.");
		dispatch(beginLibraryChange());
		let generation = getState().app.libraryGeneration;
		try {
			const userInformation = await change?.(dispatch);
			dispatch(libraryChanged());
			generation = getState().app.libraryGeneration;
			await loadAppState(dispatch, navigate, userInformation || undefined);
		} finally {
			if (getState().app.libraryGeneration === generation)
				dispatch(finishLibraryChange());
		}
	};
}

/** The backend reconnects to the selected directory before its views reload. */
export function changeDatabaseDirectory(
	directory: string,
	navigate: NavigateFunction,
) {
	return reloadApplicationState(navigate, async dispatch => {
		await flushPendingSaves();
		await dispatch(
			saveSettings(
				buildUpdateSettingsRequest({ baseDatabaseDirectory: directory }),
			),
		);
	});
}

/** Turns the on-screen keyboard suppression on or off, remembering the choice
 * for the next time the app starts on this device. */
export function setVirtualKeyboardSuppressedAction(suppressed: boolean) {
	return function (dispatch: AppDispatch, getState: () => RootState): void {
		if (selectIsVirtualKeyboardSuppressed(getState()) === suppressed)
			return;
		// Drop focus as well, so a keyboard that is already up goes away at
		// once instead of lingering until the user taps elsewhere.
		if (suppressed) {
			(document.activeElement as HTMLElement | null)?.blur();
		}
		saveVirtualKeyboardSuppressed(suppressed);
		dispatch(setVirtualKeyboardSuppressed(suppressed));
	};
}

async function loadAppState(
	dispatch: AppDispatch,
	navigate?: NavigateFunction,
	userInformationDto?: UserInformationDto,
) {
	const settings = await dispatch(loadSettings());

	if (navigate) {
		// The previously open element may not exist in the reloaded state
		// (e.g. after switching database directories), so clear it.
		dispatch(setCurrentElement(null));
		await navigate("/");
	}

	// Load elements in parallel with the user/sync calls below, so a slow or
	// unreachable backend never delays showing the user's local content.
	const elementTreePromise = dispatch(loadElementTree());

	if (userInformationDto) {
		saveCachedUserInformation(userInformationDto);
		dispatch(setUserInformation(userInformationDto));
	} else {
		await dispatch(loadUserState());
	}

	// Sync on app close is registered as an event by the SettingsSync component.
	if (settings?.autoSync) await dispatch(sync({ skipIfKnownOffline: true }));

	// Re-check the user's profile once more now that sync may have confirmed
	// connectivity, so a transient offline state at startup doesn't linger
	// for the rest of the session.
	if (!userInformationDto) await dispatch(loadUserState());

	await elementTreePromise;
}
