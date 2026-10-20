use rig::client::{CompletionClient, EmbeddingsClient};
use rig::embeddings::EmbeddingModel;
use rig::providers::gemini;
use rig::providers::ollama;
use rig::providers::openai;
use rig::providers::openrouter;

use crate::ai_integration::clients::multi_client::{
    multi_completion_model::MultiCompletionModel, multi_embedding_model::MultiEmbeddingModel,
};

/// Used for enum dispatching from multiple models.
pub enum MultiClient {
    Ollama(ollama::Client),
    OpenAI(openai::Client),
    OpenRouter(openrouter::Client),
    Gemini(gemini::Client),
}

impl CompletionClient for MultiClient {
    type CompletionModel = MultiCompletionModel;

    fn completion_model(&self, model: impl Into<String>) -> Self::CompletionModel {
        match self {
            MultiClient::Ollama(client) => {
                MultiCompletionModel::Ollama(client.completion_model(model))
            }
            MultiClient::OpenAI(client) => {
                MultiCompletionModel::OpenAI(client.completion_model(model))
            }
            MultiClient::OpenRouter(client) => {
                MultiCompletionModel::OpenRouter(client.completion_model(model))
            }
            MultiClient::Gemini(client) => {
                MultiCompletionModel::Gemini(client.completion_model(model))
            }
        }
    }
}

impl EmbeddingsClient for MultiClient {
    type EmbeddingModel = MultiEmbeddingModel;

    fn embedding_model(&self, model: impl Into<String>) -> Self::EmbeddingModel {
        MultiEmbeddingModel::make(self, model, None)
    }

    fn embedding_model_with_ndims(
        &self,
        model: impl Into<String>,
        ndims: usize,
    ) -> Self::EmbeddingModel {
        MultiEmbeddingModel::make(self, model, Some(ndims))
    }
}
