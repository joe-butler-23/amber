import {
	createAiPrompt,
	deleteAiPrompt,
	listAiPrompts,
	updateAiPrompt,
} from "../../api/aiIntegration/api/aiApi";
import AiPromptDto from "../../api/aiIntegration/dto/aiPromptDto";
import AiPromptRequestDto from "../../api/aiIntegration/dto/aiPromptRequestDto";
import { AppDispatch, RootState } from "../store";
import {
	removeAiPrompt,
	setAiPrompts,
	upsertAiPrompt,
} from "./aiPromptsReducer";
import { selectAiPromptsLoaded } from "./aiPromptsSelectors";

export function loadAiPromptsAction() {
	return async (dispatch: AppDispatch, getState: () => RootState) => {
		if (getState().app.libraryChanging || selectAiPromptsLoaded(getState())) return;
		const generation = getState().app.libraryGeneration;
		const prompts = await listAiPrompts();
		if (getState().app.libraryGeneration === generation)
			dispatch(setAiPrompts(prompts));
	};
}

/** Creates the prompt when `id` is null, otherwise updates it. */
export function saveAiPromptAction(id: string | null, dto: AiPromptRequestDto) {
	return async (dispatch: AppDispatch): Promise<AiPromptDto> => {
		const saved = id
			? await updateAiPrompt(id, dto)
			: await createAiPrompt(dto);
		dispatch(upsertAiPrompt(saved));
		return saved;
	};
}

export function deleteAiPromptAction(id: string) {
	return async (dispatch: AppDispatch) => {
		await deleteAiPrompt(id);
		dispatch(removeAiPrompt(id));
	};
}
