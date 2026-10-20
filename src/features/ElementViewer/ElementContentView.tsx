import { useRef } from "react";
import { Container, Divider, Group, Stack, Text } from "@mantine/core";
import { QuestionIcon, CheckCircleIcon } from "@phosphor-icons/react";
import { LexicalEditor } from "lexical";
import {
	CardResponseDto,
	ExtractResponseDto,
} from "../../api/elements/dto/anyElementDto";
import { updateCard, updateExtract } from "../../api/elements/api/elementsApi";
import { FloatingMenuItem } from "../../components/Editor/plugins/FloatingMenuPlugin";
import { HighlightCreatedPayload } from "../../components/Editor/plugins/HighlightPlugin/highlightCommands";
import useAppSelector from "../../hooks/useAppSelector";
import {
	selectStudyCardPhase,
	selectStudyStatus,
} from "../../stores/study/studySelectors";
import ContentOriginPanel from "./ContentOriginPanel";
import ElementEditor from "./ElementEditor";
import ExtractAiCardsButton from "./ExtractAiCardsButton";
import { useEditorFindInPage } from "./hooks/useEditorFindInPage";
import { useLineWidthContainerProps } from "./hooks/useLineWidthContainerProps";

interface Props {
	element: CardResponseDto | ExtractResponseDto;
	buttons: FloatingMenuItem[];
	onHighlightCreated: (payload: HighlightCreatedPayload) => void;
}

export default function ElementContentView({
	element,
	buttons,
	onHighlightCreated,
}: Props) {
	const editorRef = useRef<LexicalEditor | null>(null);
	const studying = useAppSelector(selectStudyStatus) === "studying";
	const question = useAppSelector(selectStudyCardPhase) === "question";
	const isCard = "front" in element;
	const fields = isCard
		? (["front", "back"] as const)
		: (["content"] as const);
	const visibleFields = fields.filter(
		field => field !== "back" || !studying || !question,
	);
	const { query, caseSensitive, onMatches, matchIndexFor } =
		useEditorFindInPage(visibleFields);
	const containerProps = useLineWidthContainerProps();

	async function save(field: "front" | "back" | "content", content: string) {
		const id = element.meta.elementId.id;
		if (field === "content") await updateExtract({ id, content });
		else await updateCard({ id, [field]: content });
	}

	return (
		<Container {...containerProps} py="lg">
			{!isCard && <ExtractAiCardsButton editorRef={editorRef} />}
			<Stack gap="xl">
				{fields.map(field => (
					<Stack key={field} gap="md">
						{field === "back" && <Divider />}
						{isCard && (
							<Group gap="xs">
								{field === "front" ? (
									<QuestionIcon size={18} />
								) : (
									<CheckCircleIcon size={18} />
								)}
								<Text size="sm" c="dimmed" tt="uppercase">
									{field}
								</Text>
							</Group>
						)}
						{field === "back" && studying && question ? (
							<Text size="md" c="dimmed" fs="italic">
								Try to recall the answer, then reveal it here.
							</Text>
						) : (
							<ElementEditor
								saveKey={`${element.meta.elementId.type}:${element.meta.elementId.id}:${field}`}
								initialContent={
									field === "content"
										? (element as ExtractResponseDto)
												.content
										: (element as CardResponseDto)[field]
								}
								buttons={buttons}
								onChange={content => save(field, content)}
								onHighlightCreated={onHighlightCreated}
								editorRef={!isCard ? editorRef : undefined}
								search={{
									editorKey: field,
									query,
									caseSensitive,
									currentMatchLocalIndex:
										matchIndexFor(field),
									onMatches,
								}}
							/>
						)}
					</Stack>
				))}
			</Stack>
			<ContentOriginPanel meta={element.meta} />
		</Container>
	);
}
