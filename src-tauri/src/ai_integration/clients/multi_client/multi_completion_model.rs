use rig::completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse};
use rig::providers::{gemini, ollama, openai, openrouter};
use rig::streaming::StreamingCompletionResponse;

/// Used for enum dispatching from multiple models.
#[derive(Clone)]
pub enum MultiCompletionModel {
    Ollama(ollama::CompletionModel),
    OpenAI(openai::responses_api::ResponsesCompletionModel),
    OpenRouter(openrouter::CompletionModel),
    Gemini(gemini::completion::CompletionModel),
}

impl CompletionModel for MultiCompletionModel {
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, CompletionError> {
        match self {
            Self::Ollama(completion_model) => completion_model.completion(request).await,
            Self::OpenAI(completion_model) => {
                completion_model
                    .completion(without_temperature(request))
                    .await
            }
            Self::OpenRouter(completion_model) => completion_model.completion(request).await,
            Self::Gemini(completion_model) => completion_model.completion(request).await,
        }
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse, CompletionError> {
        match self {
            Self::Ollama(completion_model) => completion_model.stream(request).await,
            Self::OpenAI(completion_model) => {
                completion_model.stream(without_temperature(request)).await
            }
            Self::OpenRouter(completion_model) => completion_model.stream(request).await,
            Self::Gemini(completion_model) => completion_model.stream(request).await,
        }
    }

    fn capabilities(&self) -> rig::completion::ProviderCapabilities {
        match self {
            Self::Ollama(completion_model) => completion_model.capabilities(),
            Self::OpenAI(completion_model) => completion_model.capabilities(),
            Self::OpenRouter(completion_model) => completion_model.capabilities(),
            Self::Gemini(completion_model) => completion_model.capabilities(),
        }
    }
}

/// OpenAI's reasoning models reject `temperature`, so it's left at the model's default.
fn without_temperature(request: CompletionRequest) -> CompletionRequest {
    CompletionRequest {
        temperature: None,
        ..request
    }
}
