use std::collections::HashMap;
use std::sync::Arc;

use injector_derive::ScopeInjectable;
use rig::streaming::StreamedUserContent;
use rig::{
    agent::{MultiTurnStreamItem, StreamingError, Text},
    completion::{CompletionError, PromptError},
    http_client::Error as HttpClientError,
    streaming::{StreamedAssistantContent, StreamingChat},
};
use serde::Serialize;
use thiserror::Error;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::ai_integration::ai_state::AiState;
use crate::ai_integration::dto::stream_ai_request_dto::StreamAiRequestDto;
use crate::ai_integration::entities::chat::Chat;
use crate::ai_integration::entities::context_snippet::{ContextSnippet, group_snippets_by_message};
use crate::ai_integration::entities::message::{
    Message, MessageContent, ToolApproval, ToolCallContent, ToolResultContent,
};
use crate::ai_integration::prompts::{approval_result_text, with_context_snippets};
use crate::ai_integration::repositories::ai_repository::AiRepository;
use crate::ai_integration::services::agent_provider::{
    AgentProvider, AgentProviderError, AppAgent,
};
use crate::ai_integration::services::ai_client_provider::AiClientProviderError;
use crate::ai_integration::services::chat_creator::{ChatCreator, ChatCreatorError};
use crate::ai_integration::services::tool_approval_resolver::{
    ToolApprovalResolver, ToolApprovalResolverError,
};
use crate::ai_integration::state_cancellation_hook::StateCancellationHook;
use crate::common::repository_error::RepositoryError;
use crate::database::transaction_manager::{TransactionManager, TransactionManagerError};

#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum StreamLlmResponseEvent {
    CreatedChat(Chat),
    InProgress {
        chat_id: Uuid,
        text: String,
    },
    ToolCall {
        chat_id: Uuid,
        tool_call: ToolCallContent,
        /// Already persisted, so a call awaiting approval can be resolved mid-stream.
        message_id: Uuid,
        /// Written by the tool for a call awaiting approval; never stored.
        #[serde(skip_serializing_if = "Option::is_none")]
        approval_markdown: Option<String>,
    },
    ToolResult {
        chat_id: Uuid,
        tool_result: ToolResultContent,
    },
    Error(String),
}

#[derive(Error, Debug)]
pub enum OnEventCallbackError {
    #[error("An unknown error occurred: {0}")]
    Tauri(#[from] tauri::Error),
}

pub type OnEventCallback =
    Arc<dyn Send + Sync + Fn(StreamLlmResponseEvent) -> Result<(), OnEventCallbackError>>;

#[derive(Error, Debug)]
pub enum AiStreamerError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error("HTTP {status}: {message}")]
    ProviderHttpError { status: u16, message: String },
    #[error("{0}")]
    ProviderError(String),
    #[error(transparent)]
    AiClientProvider(#[from] AiClientProviderError),
    #[error(transparent)]
    OnEventCallback(#[from] OnEventCallbackError),
    #[error(transparent)]
    ChatCreator(#[from] ChatCreatorError),
    #[error(transparent)]
    AgentProvider(#[from] AgentProviderError),
    #[error(transparent)]
    TransactionManager(#[from] TransactionManagerError),
    #[error(transparent)]
    ToolApprovalResolver(#[from] ToolApprovalResolverError),
}

impl TryFrom<CompletionError> for AiStreamerError {
    type Error = CompletionError;

    fn try_from(err: CompletionError) -> Result<Self, Self::Error> {
        match err {
            CompletionError::HttpError(HttpClientError::InvalidStatusCodeWithMessage(
                status,
                body,
            )) => {
                let message = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v["error"]["message"].as_str().map(String::from))
                    .unwrap_or(body);
                Ok(AiStreamerError::ProviderHttpError {
                    status: status.as_u16(),
                    message,
                })
            }
            CompletionError::ProviderError(message) => Ok(AiStreamerError::ProviderError(message)),
            other => Err(other),
        }
    }
}

#[derive(ScopeInjectable)]
pub struct AiStreamer {
    state: Arc<AiState>,
    ai_repository: Arc<dyn AiRepository>,
    chat_creator: Arc<ChatCreator>,
    agent_provider: Arc<AgentProvider>,
    tool_approval_resolver: Arc<ToolApprovalResolver>,
    transaction_manager: Arc<dyn TransactionManager>,
}

impl AiStreamer {
    pub async fn stream(
        &self,
        request: StreamAiRequestDto,
        on_event: OnEventCallback,
    ) -> Result<(), AiStreamerError> {
        let _guard = self.state.start_generation().await;

        let messages;
        let chat_id;
        let context_snippets_by_message: HashMap<Uuid, Vec<String>>;
        let mut chat_to_upsert = None;
        if let Some(request_chat_id) = request.chat_id {
            chat_id = request_chat_id;
            messages = self
                .ai_repository
                .get_chat_messages_ordered(chat_id)
                .await?;
            context_snippets_by_message = group_snippets_by_message(
                self.ai_repository
                    .get_context_snippets_for_chat(chat_id)
                    .await?,
            );
        } else {
            let chat = match self.chat_creator.create_chat(&request.prompt).await {
                Ok(chat) => chat,
                // Cancelling before a chat title exists yet has nothing to
                // roll back — just stop like a mid-stream cancellation does.
                Err(ChatCreatorError::Cancelled) => return Ok(()),
                Err(err) => return Err(err.into()),
            };
            chat_id = chat.id();
            messages = Vec::new();
            context_snippets_by_message = HashMap::new();
            on_event(StreamLlmResponseEvent::CreatedChat(chat.clone()))?;
            chat_to_upsert = Some(chat);
        }

        let human_message =
            Message::new(None, chat_id, MessageContent::Human(request.prompt.clone()));
        let context_snippets_to_upsert: Vec<ContextSnippet> = request
            .context_snippets
            .iter()
            .enumerate()
            .map(|(position, snippet)| {
                ContextSnippet::new(None, human_message.id(), snippet.clone(), position as i64)
            })
            .collect();

        let mut messages_to_upsert = vec![human_message];

        // A cancellation while building the agent (e.g. during the embeddings
        // dimension probe, which has no hook to check mid-flight) has no
        // completion to stream — skip straight to persisting what already
        // exists, same as a mid-stream cancellation falls through below.
        let agent = match self
            .agent_provider
            .get_agent(chat_id, &messages, request.element_id)
            .await
        {
            Ok(agent) => Some(agent),
            Err(AgentProviderError::AiClientProvider(AiClientProviderError::Cancelled)) => None,
            Err(err) => return Err(err.into()),
        };

        // The transaction resolved above (for the initial reads and agent
        // setup) would otherwise stay open — and its snapshot stale — for the
        // whole LLM round-trip, so committing it here before the potentially
        // long streaming loop avoids a SQLITE_BUSY_SNAPSHOT when the deferred
        // writes below finally run against a fresh transaction.
        self.transaction_manager.save_changes().await?;

        let mut complete_ai_response = String::new();
        let mut failed = false;
        let mut called_tools = false;

        if let Some(AppAgent {
            agent,
            approval_hook,
            tools_context,
        }) = agent
        {
            let rig_messages: Vec<rig::message::Message> = merge_consecutive_assistant_messages(
                with_current_approval_statuses(messages)
                    .into_iter()
                    .filter_map(|m| {
                        let snippets = context_snippets_by_message
                            .get(&m.id())
                            .cloned()
                            .unwrap_or_default();
                        m.try_into_rig_message(&snippets).ok()
                    })
                    .collect(),
            );
            let mut stream = agent
                .stream_chat(
                    with_context_snippets(&request.prompt, &request.context_snippets),
                    rig_messages,
                )
                .add_hook(StateCancellationHook::new(self.state.clone()))
                .add_hook(approval_hook.clone())
                .await;

            // A running tool may be mid-write, so cancelling only interrupts the wait for the model.
            let mut running_tools = 0usize;
            loop {
                let next = if running_tools > 0 {
                    stream.next().await
                } else {
                    tokio::select! {
                        next = stream.next() => next,
                        () = self.state.cancelled() => break,
                    }
                };
                let Some(content) = next else { break };
                match content {
                    Ok(content) => {
                        if let MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(Text { text, .. }),
                        ) = content
                        {
                            complete_ai_response.push_str(&text);
                            on_event(StreamLlmResponseEvent::InProgress { chat_id, text })?;
                        } else if let MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall {
                                tool_call,
                                internal_call_id,
                            },
                        ) = content
                        {
                            log::info!("Tool call: {:#?}", tool_call);
                            called_tools = true;
                            running_tools += 1;

                            let mut tool_call_content: ToolCallContent = tool_call.into();
                            // Some providers (e.g. Ollama, whose chat API has
                            // no tool-call id concept) always hand back an
                            // empty id, which would otherwise make every call
                            // in a chat indistinguishable to consumers pairing
                            // calls with results by id. Fall back to rig's own
                            // `internal_call_id`, which it generates uniquely
                            // per call regardless of provider.
                            if tool_call_content.id.is_empty() {
                                tool_call_content.id = internal_call_id;
                            }
                            if approval_hook.requires_approval(&tool_call_content.name) {
                                tool_call_content.approval =
                                    Some(ToolApproval::pending(tools_context.clone()));
                            }

                            let message = Message::new(
                                None,
                                chat_id,
                                MessageContent::ToolCall(tool_call_content.clone()),
                            );
                            let message_id = message.id();
                            messages_to_upsert.push(message);
                            self.persist(&mut chat_to_upsert, &mut messages_to_upsert)
                                .await?;
                            self.transaction_manager.save_changes().await?;

                            let approval_markdown = self
                                .tool_approval_resolver
                                .approval_markdown(&tool_call_content)
                                .await?
                                .map(|markdown| markdown.request);
                            on_event(StreamLlmResponseEvent::ToolCall {
                                chat_id,
                                tool_call: tool_call_content,
                                message_id,
                                approval_markdown,
                            })?;
                        } else if let MultiTurnStreamItem::StreamUserItem(
                            StreamedUserContent::ToolResult {
                                tool_result,
                                internal_call_id,
                            },
                        ) = content
                        {
                            log::info!("Tool result: {:#?}", tool_result);
                            running_tools = running_tools.saturating_sub(1);

                            let mut tool_result_content: ToolResultContent = tool_result.into();
                            if tool_result_content.id.is_empty() {
                                tool_result_content.id = internal_call_id;
                            }
                            on_event(StreamLlmResponseEvent::ToolResult {
                                chat_id,
                                tool_result: tool_result_content.clone(),
                            })?;
                            messages_to_upsert.push(Message::new(
                                None,
                                chat_id,
                                MessageContent::ToolResult(tool_result_content),
                            ));
                            self.transaction_manager.save_changes().await?;
                        }
                    }
                    Err(err) => {
                        log::error!("Error happened while streaming {:?}", err);

                        let is_cancelled = matches!(&err, StreamingError::Prompt(p) if matches!(**p, PromptError::PromptCancelled { .. }));

                        if !is_cancelled {
                            failed = true;
                            let error_message = match err {
                                StreamingError::Completion(completion_err) => {
                                    AiStreamerError::try_from(completion_err)
                                        .map_or_else(|e| e.to_string(), |e| e.to_string())
                                }
                                StreamingError::Prompt(prompt_err) => match *prompt_err {
                                    PromptError::CompletionError(completion_err) => {
                                        AiStreamerError::try_from(completion_err)
                                            .map_or_else(|e| e.to_string(), |e| e.to_string())
                                    }
                                    other => other.to_string(),
                                },
                            };
                            on_event(StreamLlmResponseEvent::Error(error_message))?;
                        }
                        break;
                    }
                };
            }
        }

        if !complete_ai_response.trim().is_empty() {
            messages_to_upsert.push(Message::new(
                None,
                chat_id,
                MessageContent::Assistant(complete_ai_response),
            ));
        }

        // Delaying database operations to the end to avoid the writes from locking
        // the database.
        // A failed turn's messages are dropped so the user can resend the prompt, unless
        // a tool already ran and its side effects (e.g. a created card) need recording.
        if failed && !called_tools {
            return self.persist(&mut chat_to_upsert, &mut Vec::new()).await;
        }

        self.persist(&mut chat_to_upsert, &mut messages_to_upsert)
            .await?;

        for snippet in &context_snippets_to_upsert {
            self.ai_repository.upsert_context_snippet(snippet).await?;
        }

        Ok(())
    }

    /// Writes the chat (if new) and the queued messages, draining the queue.
    async fn persist(
        &self,
        chat_to_upsert: &mut Option<Chat>,
        messages: &mut Vec<Message>,
    ) -> Result<(), AiStreamerError> {
        if let Some(chat) = chat_to_upsert.take() {
            self.ai_repository.upsert_chat(&chat).await?;
        }
        for message in messages.drain(..) {
            self.ai_repository.upsert_message(&message).await?;
        }
        Ok(())
    }
}

/// Rewrites the result of each call awaiting approval to where it now stands, so the
/// model knows which calls the user approved or rejected since.
fn with_current_approval_statuses(mut messages: Vec<Message>) -> Vec<Message> {
    let approvals: HashMap<String, ToolApproval> = messages
        .iter()
        .filter_map(|message| match message.content() {
            MessageContent::ToolCall(ToolCallContent {
                id,
                approval: Some(approval),
                ..
            }) => Some((id.clone(), approval.clone())),
            _ => None,
        })
        .collect();

    for message in &mut messages {
        if let MessageContent::ToolResult(result) = message.content_mut()
            && let Some(approval) = approvals.get(&result.id)
        {
            result.text = approval_result_text(approval.status, approval.output.as_deref());
        }
    }

    messages
}

/// A single model turn's tool calls stream in as separate `ToolCall` items
/// before any of their `ToolResult`s arrive, so each ends up persisted as its
/// own `MessageContent::ToolCall` message. Left as-is, replaying that history
/// converts to consecutive `rig::message::Message::Assistant` entries — which
/// violates providers' requirement that a tool-calling assistant message be
/// immediately followed by the matching tool responses. Merging them back
/// into one assistant message (with all of that turn's tool calls) restores
/// the shape the provider expects.
fn merge_consecutive_assistant_messages(
    messages: Vec<rig::message::Message>,
) -> Vec<rig::message::Message> {
    let mut merged: Vec<rig::message::Message> = Vec::with_capacity(messages.len());

    for message in messages {
        if let (Some(rig::message::Message::Assistant { content: prev, .. }), true) = (
            merged.last_mut(),
            matches!(message, rig::message::Message::Assistant { .. }),
        ) {
            let rig::message::Message::Assistant { content, .. } = message else {
                unreachable!()
            };
            for item in content {
                prev.push(item);
            }
        } else {
            merged.push(message);
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_integration::entities::message::ApprovalStatus;
    use rig::message::{AssistantContent, Message as RigMessage, UserContent};

    #[test]
    fn with_current_approval_statuses_approved_call_rewrote_its_tool_result_to_its_output() {
        // Arrange

        let chat_id = Uuid::new_v4();
        let messages = vec![
            Message::new(
                None,
                chat_id,
                MessageContent::ToolCall(ToolCallContent {
                    id: "tc-1".to_string(),
                    name: "create_card".to_string(),
                    arguments: serde_json::json!({}),
                    approval: Some(ToolApproval {
                        context: serde_json::Value::Null,
                        status: ApprovalStatus::Approved,
                        output: Some("Card created.".to_string()),
                    }),
                }),
            ),
            Message::new(
                None,
                chat_id,
                MessageContent::ToolResult(ToolResultContent {
                    id: "tc-1".to_string(),
                    name: "create_card".to_string(),
                    text: approval_result_text(ApprovalStatus::Pending, None),
                }),
            ),
        ];

        // Act

        let actual = with_current_approval_statuses(messages);

        // Assert

        let MessageContent::ToolResult(result) = actual[1].content() else {
            panic!("Expected a tool result message");
        };
        assert_eq!(
            approval_result_text(ApprovalStatus::Approved, Some("Card created.")),
            result.text
        );
    }

    #[test]
    fn merge_consecutive_assistant_messages_multiple_tool_calls_merged_into_one_assistant_message()
    {
        // Arrange

        let messages = vec![
            RigMessage::User {
                content: vec![UserContent::text("Make three cards")],
            },
            RigMessage::Assistant {
                id: None,
                content: vec![AssistantContent::tool_call(
                    "call-1",
                    "create_card",
                    serde_json::json!({}),
                )],
            },
            RigMessage::Assistant {
                id: None,
                content: vec![AssistantContent::tool_call(
                    "call-2",
                    "create_card",
                    serde_json::json!({}),
                )],
            },
            RigMessage::Assistant {
                id: None,
                content: vec![AssistantContent::tool_call(
                    "call-3",
                    "create_card",
                    serde_json::json!({}),
                )],
            },
            RigMessage::User {
                content: vec![UserContent::tool_result(
                    "call-1",
                    "create_card",
                    vec![rig::message::ToolResultContent::text("ok")],
                )],
            },
            RigMessage::User {
                content: vec![UserContent::tool_result(
                    "call-2",
                    "create_card",
                    vec![rig::message::ToolResultContent::text("ok")],
                )],
            },
            RigMessage::User {
                content: vec![UserContent::tool_result(
                    "call-3",
                    "create_card",
                    vec![rig::message::ToolResultContent::text("ok")],
                )],
            },
        ];

        // Act

        let actual = merge_consecutive_assistant_messages(messages);

        // Assert

        assert_eq!(5, actual.len());
        let RigMessage::Assistant { content, .. } = &actual[1] else {
            panic!("Expected an assistant message");
        };
        assert_eq!(3, content.len());
        for (item, expected_id) in content.iter().zip(["call-1", "call-2", "call-3"]) {
            let AssistantContent::ToolCall(tool_call) = item else {
                panic!("Expected a tool call");
            };
            assert_eq!(expected_id, tool_call.id.as_str());
        }
    }

    #[test]
    fn merge_consecutive_assistant_messages_no_consecutive_assistants_left_messages_unchanged() {
        // Arrange

        let messages = vec![
            RigMessage::User {
                content: vec![UserContent::text("Hi")],
            },
            RigMessage::Assistant {
                id: None,
                content: vec![AssistantContent::text("Hello")],
            },
        ];

        // Act

        let actual = merge_consecutive_assistant_messages(messages.clone());

        // Assert

        assert_eq!(messages, actual);
    }
}
