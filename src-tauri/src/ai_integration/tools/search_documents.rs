use std::sync::Arc;

use rig::sqlite::SqliteSearchFilter;
use rig::{
    tool::{Tool, ToolContext},
    vector_store::{
        VectorSearchRequest, VectorStoreError, VectorStoreIndex, request::SearchFilter,
    },
};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::ai_integration::{
    entities::{
        document::{CHAT_ID_COLUMN_NAME, Document},
        message::{Message, MessageContent},
    },
    services::ai_client_provider::{AiClientProvider, AiClientProviderError},
    tools::app_tool::AppTool,
};

#[derive(Deserialize, Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct SearchDocumentsArgs {
    #[schemars(
        description = "The search query or question to find relevant information in the uploaded files."
    )]
    pub query: String,
    #[schemars(description = "The maximum number of top matching results to return.")]
    pub top_k: u64,
}

#[derive(Error, Debug)]
pub enum SearchDocumentsError {
    #[error("Failed to fetch documents from the vector store")]
    Fetching(#[from] VectorStoreError),
    #[error(transparent)]
    AiClientProvider(#[from] AiClientProviderError),
}

pub struct SearchDocuments {
    chat_id: Uuid,
    ai_client_provider: Arc<dyn AiClientProvider>,
}

impl SearchDocuments {
    pub fn new(ai_client_provider: Arc<dyn AiClientProvider>, chat_id: Uuid) -> Self {
        Self {
            ai_client_provider,
            chat_id,
        }
    }
}

impl Tool for SearchDocuments {
    const NAME: &'static str = "search_documents";

    type Error = SearchDocumentsError;
    type Args = SearchDocumentsArgs;
    type Output = Vec<Document>;

    fn description(&self) -> String {
        "Performs semantic search over the text content of \
            all files uploaded by the user. It returns relevant \
            snippets (chunks) that match the query"
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schema_for!(SearchDocumentsArgs)).unwrap()
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let client = self.ai_client_provider.get_client().await?;
        let embed_model = self
            .ai_client_provider
            .get_embeddings_model(&client)
            .await?;
        let vector_store = self
            .ai_client_provider
            .get_vector_store(&embed_model)
            .await?;
        let index = vector_store.index(embed_model);

        let filter = SqliteSearchFilter::eq(
            CHAT_ID_COLUMN_NAME,
            serde_json::to_value(self.chat_id.to_string()).unwrap(),
        );

        let req = VectorSearchRequest::builder()
            .samples(args.top_k)
            .query(args.query)
            .filter(filter)
            .build();

        let results = index
            .top_n::<Document>(req)
            .await?
            .into_iter()
            .map(|(_, _, document)| document)
            .collect::<Vec<_>>();

        Ok(results)
    }
}

impl AppTool for SearchDocuments {
    const REQUIRES_APPROVAL: bool = false;

    /// Only chats with an uploaded document have anything to search.
    fn is_applicable(&self, messages: &[Message]) -> bool {
        messages
            .iter()
            .any(|m| matches!(m.content(), MessageContent::Document(_)))
    }
}
