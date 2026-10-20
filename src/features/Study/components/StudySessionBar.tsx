import { useEffect, useRef, useState } from "react";
import { Box, Button, Group, Text } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useNavigate } from "react-router";
import { SMALL_SCREEN_BREAKPOINT } from "../../../hooks/useIsSmallScreen";
import { previewNextLearningAsset } from "../../../api/study/api/studyApi";
import useAppDispatch from "../../../hooks/useAppDispatch";
import useAppSelector from "../../../hooks/useAppSelector";
import useApi from "../../../hooks/useApi";
import errorToString from "../../../utils/errorToString";
import { useCardScheduling } from "../hooks/useCardScheduling";
import { useElapsedSeconds } from "../hooks/useElapsedSeconds";
import {
	finishLearningAssetAction,
	gradeCardAction,
	nextLearningAssetAction,
	skipLearningAssetAction,
} from "../../../stores/study/studyActions";
import { answerShown, elementShown } from "../../../stores/study/studyReducer";
import {
	selectStudyCardPhase,
	selectStudyCurrentElement,
	selectStudyQueue,
	selectStudyShownAt,
	selectStudyTotalCount,
} from "../../../stores/study/studySelectors";
import { Rating } from "../../../types/study/rating";
import { formatRelativeDueDate } from "../../../utils/formatRelativeDueDate";
import AppTooltip from "../../../components/AppTooltip/AppTooltip";
import { useAppHotkeys } from "../../../commands/useAppHotkeys";
import { useRunCommand } from "../../../commands/useRunCommand";
import { flushPendingSaves } from "../../ElementViewer/hooks/useAutoSave";
import {
	FINISH_LEARNING_ASSET_SHORTCUT,
	GRADE_AGAIN_SHORTCUT,
	GRADE_EASY_SHORTCUT,
	GRADE_GOOD_SHORTCUT,
	GRADE_HARD_SHORTCUT,
	NEXT_LEARNING_ASSET_SHORTCUT,
	PICK_DUE_DATE_SHORTCUT,
	SHOW_ANSWER_SHORTCUT,
	SKIP_LEARNING_ASSET_SHORTCUT,
} from "../../../config/shortcuts";

const RATINGS: [Rating, string, string][] = [
	["again", "Again", GRADE_AGAIN_SHORTCUT],
	["hard", "Hard", GRADE_HARD_SHORTCUT],
	["good", "Good", GRADE_GOOD_SHORTCUT],
	["easy", "Easy", GRADE_EASY_SHORTCUT],
];

export default function StudySessionBar() {
	const dispatch = useAppDispatch();
	const navigate = useNavigate();
	const runCommand = useRunCommand();
	const current = useAppSelector(selectStudyCurrentElement);
	const queue = useAppSelector(selectStudyQueue);
	const total = useAppSelector(selectStudyTotalCount);
	const phase = useAppSelector(selectStudyCardPhase);
	const shownAt = useAppSelector(selectStudyShownAt);
	const elapsed = useElapsedSeconds(shownAt);
	const { callApi, errorMessage, isSendingRequest } = useApi();
	const busy = useRef(false);
	const first = useRef(true);
	const key = current ? `${current.type}:${current.id}` : null;
	const [nextDue, setNextDue] = useState<{
		key: string | null;
		due: string;
	} | null>(null);
	const {
		preview,
		schedule,
		errorMessage: scheduleError,
	} = useCardScheduling(
		current?.type === "card" ? current.id : null,
		shownAt,
	);

	useEffect(() => {
		if (!current) return;
		if (first.current) first.current = false;
		else dispatch(elementShown());
		let cancelled = false;
		if (current.type !== "card")
			void previewNextLearningAsset(current)
				.then(due => {
					if (!cancelled) setNextDue({ key, due });
				})
				.catch(error =>
					notifications.show({
						message: errorToString(error),
						color: "red",
					}),
				);
		return () => {
			cancelled = true;
		};
	}, [current, key, dispatch]);

	const hidden = current?.type === "card" && phase === "question";
	const actions = !current
		? []
		: hidden
			? [
					{
						kind: "show" as const,
						label: "Show answer",
						shortcut: SHOW_ANSWER_SHORTCUT,
						hint: undefined,
					},
				]
			: current.type === "card"
				? RATINGS.map(([rating, label, shortcut]) => ({
						kind: rating,
						label,
						shortcut,
						hint: preview
							? formatRelativeDueDate(preview[rating].due)
							: undefined,
					}))
				: [
						{
							kind: "skip" as const,
							label: "Skip",
							shortcut: SKIP_LEARNING_ASSET_SHORTCUT,
							hint: "Move to the end of the queue",
						},
						{
							kind: "next" as const,
							label: "Next",
							shortcut: NEXT_LEARNING_ASSET_SHORTCUT,
							hint:
								nextDue?.key === key
									? formatRelativeDueDate(nextDue.due)
									: undefined,
						},
						{
							kind: "finish" as const,
							label: "Finish",
							shortcut: FINISH_LEARNING_ASSET_SHORTCUT,
							hint: "Won't repeat",
						},
					];

	function handleAction(kind: (typeof actions)[number]["kind"]) {
		if (!current || busy.current) return;
		if (kind === "show") {
			dispatch(answerShown());
			return;
		}
		const rating = RATINGS.find(([value]) => value === kind)?.[0];
		const review = rating && schedule(rating);
		const action =
			current.type === "card"
				? rating &&
					review &&
					gradeCardAction(current.id, review, rating, navigate)
				: kind === "skip"
					? skipLearningAssetAction(current, navigate)
					: kind === "next"
						? nextLearningAssetAction(current, navigate)
						: kind === "finish" &&
							finishLearningAssetAction(current, navigate);
		if (!action) return;
		busy.current = true;
		void callApi(async () => {
			await flushPendingSaves();
			await dispatch(action);
		}).finally(() => {
			busy.current = false;
		});
	}

	useAppHotkeys(
		current
			? [
					...actions.map(
						action =>
							[
								action.shortcut,
								() => handleAction(action.kind),
							] as [string, () => void],
					),
					...(current.type !== "card"
						? [
								[
									PICK_DUE_DATE_SHORTCUT,
									() => runCommand("open-due-date"),
								] as [string, () => void],
							]
						: []),
				]
			: [],
	);
	if (!current) return null;

	return (
		<Group h="100%" px="sm" py="xs" wrap="nowrap" align="center">
			<Box flex={1}>
				<Text
					size="sm"
					c="dimmed"
					visibleFrom={SMALL_SCREEN_BREAKPOINT}>
					{total - queue.length + 1}/{total}
				</Text>
			</Box>
			{(errorMessage ?? scheduleError) && (
				<Text role="alert" size="xs" c="red">
					{errorMessage ?? scheduleError}
				</Text>
			)}
			<Group gap="xs" wrap="nowrap" align="flex-end">
				{actions.map(action => (
					<AppTooltip
						key={action.label}
						touch
						label={action.hint}
						shortcut={action.shortcut}>
						<Button
							size="sm"
							variant="default"
							disabled={
								isSendingRequest ||
								(!hidden && current.type === "card" && !preview)
							}
							onClick={() => handleAction(action.kind)}>
							{action.label}
						</Button>
					</AppTooltip>
				))}
			</Group>
			<Box flex={1}>
				<Text
					size="sm"
					c="dimmed"
					ta="right"
					visibleFrom={SMALL_SCREEN_BREAKPOINT}>
					{Math.floor(elapsed / 60)}:
					{(elapsed % 60).toString().padStart(2, "0")}
				</Text>
			</Box>
		</Group>
	);
}
