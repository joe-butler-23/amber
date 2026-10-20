use std::sync::Arc;

use async_trait::async_trait;
use injector_derive::ScopeInjectable;
use rig::client::AgentClientExt;
use rig::completion::PromptError;
use rig::extractor::ExtractionError;

use crate::ai_integration::ai_state::AiState;
use crate::ai_integration::entities::chat::Chat;
use crate::ai_integration::json_schemas::generate_title::GenerateTitle;
use crate::ai_integration::prompts::PREAMBLE_GENERATE_TITLE;
use crate::ai_integration::services::ai_client_provider::AiClientProvider;
use crate::ai_integration::services::chat_creator::{ChatCreator, ChatCreatorError};
use crate::ai_integration::state_cancellation_hook::StateCancellationHook;

#[derive(ScopeInjectable)]
pub struct DefaultChatCreator {
    ai_client_provider: Arc<dyn AiClientProvider>,
    state: Arc<AiState>,
}

#[async_trait]
impl ChatCreator for DefaultChatCreator {
    async fn create_chat(&self, prompt: &str) -> Result<Chat, ChatCreatorError> {
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
