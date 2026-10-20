use std::path::{Path, PathBuf};
use std::sync::Arc;

use injector_derive::ScopeInjectable;
use rig::embeddings::{EmbedError, Embedding, EmbeddingError, EmbeddingsBuilder};
use rig::loaders::file::FileLoaderError;
use rig::loaders::pdf::PdfLoaderError;
use rig::loaders::{FileLoader, PdfFileLoader};
use rig::vector_store::VectorStoreError;
use serde::Serialize;
use text_splitter::{ChunkConfig, TextSplitter};
use thiserror::Error;
use uuid::Uuid;

use crate::SourceError;
use crate::ai_integration::ai_state::AiState;
use crate::ai_integration::clients::multi_client::multi_embedding_model::MultiEmbeddingModel;
use crate::ai_integration::entities::document::Document;
use crate::ai_integration::entities::message::{self, Message, MessageContent};
use crate::ai_integration::repositories::ai_repository::AiRepository;
use crate::ai_integration::services::ai_client_provider::{AiClientProvider, AiClientProviderError};
use crate::common::repository_error::RepositoryError;

#[derive(Error, Debug)]
pub enum DocumentUploaderError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error("An unknown error occurred")]
    Unknown(#[source] SourceError),
    #[error("Failed to load the file")]
    FileLoader(#[from] FileLoaderError),
    #[error("Failed to load the PDF file")]
    PdfLoader(#[from] PdfLoaderError),
    #[error("Failed to embed content")]
    Embed(#[from] EmbedError),
    #[error("Failed to generate embeddings")]
    Embedding(#[from] EmbeddingError),
    #[error(transparent)]
    VectorStore(#[from] VectorStoreError),
    #[error(transparent)]
    AiClientProvider(#[from] AiClientProviderError),
}

/// Whether the document was stored, or the upload was stopped before anything was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UploadOutcome {
    Uploaded,
    Stopped,
}

#[derive(ScopeInjectable)]
pub struct DocumentUploader {
    ai_repository: Arc<dyn AiRepository>,
    ai_client_provider: Arc<AiClientProvider>,
    state: Arc<AiState>,
}

impl DocumentUploader {
    /// Reads, chunks and embeds the file: the slow part, which stopping can interrupt.
    async fn embed_document(
        &self,
        path: &Path,
        chat_id: Uuid,
    ) -> Result<(MultiEmbeddingModel, Vec<(Document, Vec<Embedding>)>), DocumentUploaderError> {
        let client = self.ai_client_provider.get_client().await?;
        let embed_model = self
            .ai_client_provider
            .get_embeddings_model(&client)
            .await?;

        // PDF parsing is blocking, so it runs off the async runtime to stay cancellable.
        let file_path = path.to_path_buf();
        let documents = tokio::task::spawn_blocking(move || load_documents(&file_path, chat_id))
            .await
            .map_err(|err| DocumentUploaderError::Unknown(Box::new(err)))??;

        let embeddings = EmbeddingsBuilder::new(embed_model.clone())
            .documents(documents)?
            .build()
            .await?;

        Ok((embed_model, embeddings))
    }

    pub async fn upload_document(
        &self,
        path: PathBuf,
        chat_id: Uuid,
    ) -> Result<UploadOutcome, DocumentUploaderError> {
        let _guard = self.state.start_generation().await;

        let result = tokio::select! {
            result = self.embed_document(&path, chat_id) => result,
            () = self.state.cancelled() => return Ok(UploadOutcome::Stopped),
        };
        let (embed_model, embeddings) = match result {
            Ok(embedded) => embedded,
            Err(DocumentUploaderError::AiClientProvider(AiClientProviderError::Cancelled)) => {
                return Ok(UploadOutcome::Stopped);
            }
            Err(err) => return Err(err),
        };
        // `select!` may pick the finished embedding over a stop that arrived alongside it.
        if self.state.generation_cancelled() {
            return Ok(UploadOutcome::Stopped);
        }

        let vector_store = self
            .ai_client_provider
            .get_vector_store(&embed_model)
            .await?;
        vector_store.add_rows(embeddings).await?;

        self.ai_repository
            .upsert_message(&Message::new(
                None,
                chat_id,
                MessageContent::Document(message::DocumentContent {
                    file_name: path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("")
                        .to_string(),
                }),
            ))
            .await?;

        Ok(UploadOutcome::Uploaded)
    }
}

/// Splits the file into chunks to embed: per page for PDFs, per file otherwise.
fn load_documents(path: &Path, chat_id: Uuid) -> Result<Vec<Document>, DocumentUploaderError> {
    let splitter = TextSplitter::new(ChunkConfig::new(512).with_trim(false));
    let glob = &path.to_string_lossy();
    let upload_id = Uuid::new_v4();

    if let Some(extension) = path.extension()
        && extension == "pdf"
    {
        let loader = PdfFileLoader::with_glob(glob)?;

        return Ok(loader
            .load_with_path()
            .ignore_errors()
            .by_page()
            .into_iter()
            .flat_map(|(_path, result)| result)
            .map(|(_pageno, result)| result)
            .filter_map(|v| v.ok())
            .enumerate()
            .flat_map(|(page_i, page)| {
                splitter
                    .chunks(&page)
                    .enumerate()
                    .map(move |(chunk_i, chunk)| Document {
                        id: format!("{upload_id}:page_{page_i}:chunk_{chunk_i}"),
                        content: chunk.to_string(),
                        chat_id,
                    })
                    .collect::<Vec<_>>()
            })
            .collect());
    }

    let loader = FileLoader::with_glob(glob)?;

    Ok(loader
        .read()
        .ignore_errors()
        .into_iter()
        .enumerate()
        .flat_map(|(file_i, content)| {
            splitter
                .chunks(&content)
                .enumerate()
                .map(move |(chunk_i, chunk)| Document {
                    id: format!("{upload_id}:file_{file_i}:chunk_{chunk_i}"),
                    content: chunk.to_string(),
                    chat_id,
                })
                .collect::<Vec<_>>()
        })
        .collect())
}
