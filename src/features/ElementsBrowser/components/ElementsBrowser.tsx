import { useEffect, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router";
import { useDebouncedValue } from "@mantine/hooks";
import { Alert, Container, Group, Paper, Stack } from "@mantine/core";
import PageTitle from "../../../components/PageTitle/PageTitle";
import { BrowserIcon } from "../../../config/icons";
import {
	ElementFilter,
	ElementFilterField,
} from "../../../api/savedSearches/dto/elementFilter";
import { ElementsBrowserLocationState } from "../../../types/elements/elementsBrowserLocationState";
import { StudyProfileDto } from "../../../api/study/dto/studyProfileDto";
import { listStudyProfiles } from "../../../api/study/api/studyProfileApi";
import { SavedSearchResponseDto } from "../../../api/savedSearches/dto/savedSearchResponseDto";
import { listSavedSearches } from "../../../api/savedSearches/api/savedSearchesApi";
import { searchElements } from "../../../api/search/api/searchApi";
import { SearchElementResultDto } from "../../../api/search/dto/searchElementResultDto";
import { ElementId } from "../../../types/elements/elementId";
import useApi from "../../../hooks/useApi";
import useAppSelector from "../../../hooks/useAppSelector";
import useAppDispatch from "../../../hooks/useAppDispatch";
import { selectBibliographicalSources } from "../../../stores/bibliographicalSources/bibliographicalSourcesSelectors";
import { loadBibliographicalSourcesAction } from "../../../stores/bibliographicalSources/bibliographicalSourcesActions";
import { selectSyncRevision } from "../../../stores/sync/syncSelector";
import { SEARCH_DEBOUNCE_MS } from "../config/constants";
import { createDefaultFilter } from "../utils/createDefaultFilter";
import { elementKey } from "../../../utils/elementKey";
import { DEFAULT_SEARCH_SORT } from "../utils/searchSort";
import { SearchSortDto } from "../../../api/search/dto/searchSortDto";
import FilterChip from "./Filter/FilterChip";
import AddFilterMenu from "./Filter/AddFilterMenu";
import SavedSearchSelector from "./SavedSearch/SavedSearchSelector";
import SearchResultsTable from "./SearchResultsTable";
import BulkActionsBar from "./BulkActions/BulkActionsBar";

export default function ElementsBrowser() {
	const location = useLocation();
	const navigate = useNavigate();
	const locationState = location.state as ElementsBrowserLocationState | null;

	const [filters, setFilters] = useState<ElementFilter[]>(
		locationState?.elementsBrowser?.filters ?? [],
	);
	const [loadedSavedSearchId, setLoadedSavedSearchId] = useState<
		string | null
	>(locationState?.elementsBrowser?.loadedSavedSearchId ?? null);
	const [justAddedId, setJustAddedId] = useState<string | null>(null);

	const sources = useAppSelector(selectBibliographicalSources);
	const dispatch = useAppDispatch();
	const [profiles, setProfiles] = useState<StudyProfileDto[]>([]);
	const [savedSearches, setSavedSearches] = useState<
		SavedSearchResponseDto[]
	>([]);
	const [results, setResults] = useState<SearchElementResultDto[]>([]);
	const [sort, setSort] = useState<SearchSortDto>(
		locationState?.elementsBrowser?.sort ?? DEFAULT_SEARCH_SORT,
	);
	const [selectedIds, setSelectedIds] = useState<ElementId[]>(
		locationState?.elementsBrowser?.selectedIds ?? [],
	);
	const { callApi, errorMessage } = useApi();
	const syncRevision = useAppSelector(selectSyncRevision);
	const libraryGeneration = useAppSelector(state => state.app.libraryGeneration);
	const libraryChanging = useAppSelector(state => state.app.libraryChanging);
	const [debouncedFilters] = useDebouncedValue(filters, SEARCH_DEBOUNCE_MS);
	// Searches are fired on every filter edit and can resolve out of order, so
	// only the most recently started one is allowed to publish its results.
	const latestSearchId = useRef(0);

	function loadSupportingLists() {
		if (libraryChanging) return;
		void dispatch(loadBibliographicalSourcesAction());
		void listStudyProfiles().then(setProfiles);
		void listSavedSearches().then(setSavedSearches);
	}

	useEffect(() => {
		loadSupportingLists();
	}, [syncRevision, libraryGeneration, libraryChanging, dispatch]);

	function runSearch() {
		if (libraryChanging) return;
		const searchId = ++latestSearchId.current;
		void callApi(() =>
			searchElements({ filters: debouncedFilters, sort }),
		).then(searchResults => {
			if (searchResults && searchId === latestSearchId.current) {
				setResults(searchResults);
			}
		});
	}

	useEffect(() => {
		runSearch();
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [
		debouncedFilters,
		sort,
		syncRevision,
		libraryGeneration,
		libraryChanging,
	]);

	function updateFilters(next: ElementFilter[]) {
		setFilters(next);
		setSelectedIds([]);
	}

	useEffect(() => {
		void navigate(location.pathname, {
			replace: true,
			state: {
				elementsBrowser: {
					filters,
					loadedSavedSearchId,
					selectedIds,
					sort,
				},
			},
		});
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [filters, loadedSavedSearchId, selectedIds, sort]);

	function handleAddFilter(field: ElementFilterField) {
		const filter = createDefaultFilter(field);
		updateFilters([...filters, filter]);
		setJustAddedId(filter.id);
	}

	function handleChangeFilter(updated: ElementFilter) {
		updateFilters(filters.map(f => (f.id === updated.id ? updated : f)));
	}

	function handleRemoveFilter(id: string) {
		updateFilters(filters.filter(f => f.id !== id));
	}

	const selectedKeys = new Set(selectedIds.map(elementKey));
	const selectedResults = results.filter(r =>
		selectedKeys.has(elementKey(r)),
	);

	return (
		<Container size="md" py="lg">
			<Paper withBorder radius="md" p="md">
				<Stack gap="sm">
					<PageTitle
						icon={<BrowserIcon />}
						title="Browser"
						description="Search and filter every element in your collection. Save the queries you use often, and select results to act on them in bulk."
					/>
					<SavedSearchSelector
						filters={filters}
						onFiltersChange={updateFilters}
						sort={sort}
						onSortChange={setSort}
						loadedSavedSearchId={loadedSavedSearchId}
						onLoadedSavedSearchIdChange={setLoadedSavedSearchId}
						savedSearches={savedSearches}
						onSavedSearchesChange={setSavedSearches}
					/>
					<Group gap="xs" wrap="wrap">
						{filters.map(filter => (
							<FilterChip
								key={filter.id}
								filter={filter}
								sources={sources}
								profiles={profiles}
								defaultOpened={filter.id === justAddedId}
								onChange={handleChangeFilter}
								onRemove={() => handleRemoveFilter(filter.id)}
							/>
						))}
						<AddFilterMenu onSelect={handleAddFilter} />
					</Group>
					{errorMessage && (
						<Alert color="red" title="Search failed">
							{errorMessage}
						</Alert>
					)}
					<BulkActionsBar
						selectedIds={selectedIds}
						selectedResults={selectedResults}
						resultCount={results.length}
						profiles={profiles}
						sources={sources}
						onClearSelection={() => setSelectedIds([])}
						onActionComplete={runSearch}
					/>
					<SearchResultsTable
						results={results}
						sort={sort}
						onSortChange={setSort}
						selectedIds={selectedIds}
						onSelectionChange={setSelectedIds}
					/>
				</Stack>
			</Paper>
		</Container>
	);
}
