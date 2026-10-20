use std::sync::Arc;

use injector_derive::ScopeInjectable;
use rig::client::AgentClientExt;
use rig::completion::{CompletionError, PromptError};
use rig::extractor::ExtractionError;
use rig::http_client::Error as HttpClientError;
use thiserror::Error;

use crate::SourceError;
use crate::ai_integration::ai_state::AiState;
use crate::ai_integration::entities::chat::Chat;
use crate::ai_integration::json_schemas::generate_title::GenerateTitle;
use crate::ai_integration::prompts::PREAMBLE_GENERATE_TITLE;
use crate::ai_integration::services::ai_client_provider::{AiClientProvider, AiClientProviderError};
use crate::ai_integration::state_cancellation_hook::StateCancellationHook;

#[derive(Error, Debug)]
pub enum ChatCreatorError {
    #[error(transparent)]
    AiClientProvider(#[from] AiClientProviderError),
    #[error("HTTP {status}: {message}")]
    ProviderHttpError { status: u16, message: String },
    #[error("{0}")]
    ProviderError(String),
    #[error("Failed to create the chat: {0}")]
    CreateChat(#[source] SourceError),
    #[error("Chat creation was cancelled")]
    Cancelled,
}

impl TryFrom<CompletionError> for ChatCreatorError {
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
                Ok(ChatCreatorError::ProviderHttpError {
                    status: status.as_u16(),
                    message,
                })
            }
            CompletionError::ProviderError(message) => Ok(ChatCreatorError::ProviderError(message)),
            other => Err(other),
        }
    }
}

#[derive(ScopeInjectable)]
pub struct ChatCreator {
    ai_client_provider: Arc<AiClientProvider>,
    state: Arc<AiState>,
}

impl ChatCreator {
    pub async fn create_chat(&self, prompt: &str) -> Result<Chat, ChatCreatorError> {
        let extractor = self
            .ai_client_provider
            .get_client()
            .await?
            .extractor::<GenerateTitle>(self.ai_client_provider.get_completion_model_name().await?)
            .preamble(PREAMBLE_GENERATE_TITLE)
            .add_hook(StateCancellationHook::new(self.state.clone()))
            .build();
        let extraction = extractor.extract(format!("User message: {}", prompt));
        let result = tokio::select! {
            result = extraction => result,
            () = self.state.cancelled() => return Err(ChatCreatorError::Cancelled),
        };
        let response = match result {
            Ok(response) => response,
            Err(ExtractionError::CompletionError(completion_err)) => {
                return Err(ChatCreatorError::try_from(completion_err)
                    .unwrap_or_else(|e| ChatCreatorError::CreateChat(Box::new(e))));
            }
            Err(ExtractionError::PromptError(PromptError::PromptCancelled { .. })) => {
                return Err(ChatCreatorError::Cancelled);
            }
            Err(err) => return Err(ChatCreatorError::CreateChat(Box::new(err))),
        };

        log::info!("Generated title for chat is '{}'.", response.title);
        Ok(Chat::new(None, response.title))
    }
}
