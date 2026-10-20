use rig::embeddings::EmbeddingModel;
use rig::providers::gemini;
use rig::providers::ollama;
use rig::providers::openai;
use rig::providers::openrouter;

use crate::ai_integration::clients::multi_client::MultiClient;

#[derive(Clone)]
enum Model {
    Ollama(ollama::EmbeddingModel),
    OpenAI(openai::EmbeddingModel),
    OpenRouter(openrouter::EmbeddingModel),
    Gemini(gemini::embedding::EmbeddingModel),
}

#[derive(Clone)]
pub struct MultiEmbeddingModel {
    inner: Model,
    name: String,
}

impl MultiEmbeddingModel {
    pub fn identity(&self) -> (&'static str, &str) {
        let provider = match &self.inner {
            Model::Ollama(_) => "ollama",
            Model::OpenAI(_) => "openai",
            Model::OpenRouter(_) => "openrouter",
            Model::Gemini(_) => "gemini",
        };
        (provider, &self.name)
    }
}

impl EmbeddingModel for MultiEmbeddingModel {
    const MAX_DOCUMENTS: usize = 1024;

    type Client = MultiClient;

    fn make(client: &Self::Client, model: impl Into<String>, dims: Option<usize>) -> Self {
        let name = model.into();
        let model = name.clone();
        let inner = match client {
            MultiClient::Ollama(client) => {
                Model::Ollama(ollama::EmbeddingModel::make(client, model, dims))
            }
            MultiClient::OpenAI(client) => {
                Model::OpenAI(openai::EmbeddingModel::make(client, model, dims))
            }
            MultiClient::OpenRouter(client) => {
                Model::OpenRouter(openrouter::EmbeddingModel::make(client, model, dims))
            }
            MultiClient::Gemini(client) => {
                Model::Gemini(gemini::embedding::EmbeddingModel::make(client, model, dims))
            }
        };
        Self { inner, name }
    }

    fn ndims(&self) -> usize {
        match &self.inner {
            Model::Ollama(embedding_model) => embedding_model.ndims(),
            Model::OpenAI(embedding_model) => embedding_model.ndims(),
            Model::OpenRouter(embedding_model) => embedding_model.ndims(),
            Model::Gemini(embedding_model) => embedding_model.ndims(),
        }
    }

    async fn embed_texts(
        &self,
        texts: impl IntoIterator<Item = String> + rig::wasm_compat::WasmCompatSend,
    ) -> Result<Vec<rig::embeddings::Embedding>, rig::embeddings::EmbeddingError> {
        match &self.inner {
            Model::Ollama(embedding_model) => embedding_model.embed_texts(texts).await,
            Model::OpenAI(embedding_model) => embedding_model.embed_texts(texts).await,
            Model::OpenRouter(embedding_model) => embedding_model.embed_texts(texts).await,
            Model::Gemini(embedding_model) => embedding_model.embed_texts(texts).await,
        }
    }
}
