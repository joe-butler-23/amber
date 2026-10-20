import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import {
	AnyElementDto,
	MetaResponseDto,
} from "../../api/elements/dto/anyElementDto";
import { ElementDetailsResponseDto } from "../../api/elements/dto/elementDetailsDto";
import { NodeDto } from "../../api/elements/dto/nodeDto";

export interface ElementsState {
	tree: NodeDto[];
	isLoading: boolean;
	error: string | null;
	currentElement: AnyElementDto | null;
	details: ElementDetailsResponseDto | null;
	loadVersion: number;
	contentVersion: number;
	refreshContent: boolean;
	isDetailsLoading: boolean;
	/** Set by a view (e.g. the PDF viewer) that owns its own independent
	 * zoom, so app-wide zoom shortcuts/gestures can leave it alone. */
	zoomOwnedByCurrentView: boolean;
}

const initialState: ElementsState = {
	tree: [],
	isLoading: false,
	error: null,
	currentElement: null,
	details: null,
	loadVersion: 0,
	contentVersion: 0,
	refreshContent: false,
	isDetailsLoading: false,
	zoomOwnedByCurrentView: false,
};

const elementsSlice = createSlice({
	name: "elements",
	initialState,
	reducers: {
		setTreeLoading: state => {
			state.isLoading = true;
			state.error = null;
		},
		setTree: (state, action: PayloadAction<NodeDto[]>) => {
			state.tree = action.payload;
			state.isLoading = false;
			state.error = null;
		},
		setTreeError: (state, action: PayloadAction<string>) => {
			state.error = action.payload;
			state.isLoading = false;
		},
		clearTreeError: state => {
			state.error = null;
		},
		setCurrentElement: (
			state,
			action: PayloadAction<AnyElementDto | null>,
		) => {
			state.loadVersion++;
			state.currentElement = action.payload;
			state.details = null;
			state.isDetailsLoading = false;
			state.refreshContent = false;
			state.zoomOwnedByCurrentView = false;
		},
		beginElementLoad: (state, action: PayloadAction<boolean>) => {
			state.loadVersion++;
			state.refreshContent ||= action.payload;
		},
		setLoadedElement: (
			state,
			action: PayloadAction<{
				version: number;
				element: AnyElementDto;
				details: ElementDetailsResponseDto;
			}>,
		) => {
			if (state.loadVersion !== action.payload.version) return;
			state.currentElement = action.payload.element;
			state.details = action.payload.details;
			if (state.refreshContent) state.contentVersion++;
			state.refreshContent = false;
			state.isDetailsLoading = false;
			state.zoomOwnedByCurrentView = false;
		},
		setDetailsLoading: (state, action: PayloadAction<boolean>) => {
			state.isDetailsLoading = action.payload;
		},
		setCurrentElementDetails: (
			state,
			action: PayloadAction<ElementDetailsResponseDto>,
		) => {
			state.details = action.payload;
			state.isDetailsLoading = false;
		},
		setZoomOwnedByCurrentView: (state, action: PayloadAction<boolean>) => {
			state.zoomOwnedByCurrentView = action.payload;
		},
		setCurrentElementMeta: (
			state,
			action: PayloadAction<Partial<MetaResponseDto>>,
		) => {
			if (!state.currentElement) return;
			state.currentElement.data.meta = {
				...state.currentElement.data.meta,
				...action.payload,
			};
		},
	},
});

export default elementsSlice.reducer;

export const {
	setTreeLoading,
	setTree,
	setTreeError,
	clearTreeError,
	setCurrentElement,
	beginElementLoad,
	setLoadedElement,
	setDetailsLoading,
	setCurrentElementDetails,
	setCurrentElementMeta,
	setZoomOwnedByCurrentView,
} = elementsSlice.actions;
