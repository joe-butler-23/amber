use std::sync::Arc;

use async_trait::async_trait;
use injector_derive::ScopeInjectable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::ai_integration::services::ai_client_provider::AiClientProvider;
use crate::ai_integration::services::tools_provider::{ToolsProvider, ToolsProviderError};
use crate::ai_integration::tools::app_tools::AppTools;
use crate::ai_integration::tools::create_card::CreateCard;
use crate::ai_integration::tools::create_cloze_card::CreateClozeCard;
use crate::ai_integration::tools::finish_extract::FinishExtract;
use crate::ai_integration::tools::search_documents::SearchDocuments;
use crate::common::services::lexical_json_converter::LexicalJsonConverter;
use crate::elements::repositories::meta_repository::MetaRepository;
use crate::elements::services::element_creation_service::ElementCreationService;
use crate::elements::value_objects::element_id::ElementId;
use crate::study::services::learning_asset_scheduling_service::LearningAssetSchedulingService;

/// What the tools need from the conversation they were given in.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolsContext {
    chat_id: Uuid,
    element_id: Option<ElementId>,
}

#[derive(ScopeInjectable)]
pub struct DefaultToolsProvider {
    ai_client_provider: Arc<AiClientProvider>,
    meta_repository: Arc<dyn MetaRepository>,
    element_creation_service: Arc<dyn ElementCreationService>,
    lexical_json_converter: Arc<dyn LexicalJsonConverter>,
    learning_asset_scheduling_service: Arc<dyn LearningAssetSchedulingService>,
}

#[async_trait]
impl ToolsProvider for DefaultToolsProvider {
    fn capture_context(&self, chat_id: Uuid, element_id: Option<ElementId>) -> Value {
        serde_json::to_value(ToolsContext {
            chat_id,
            element_id,
        })
        .unwrap_or_default()
    }

    async fn get_tools(&self, context: &Value) -> Result<AppTools, ToolsProviderError> {
        let context: ToolsContext = serde_json::from_value(context.clone())?;

        let element = existing_element(self.meta_repository.as_ref(), context.element_id).await?;

        Ok(AppTools::default()
            .tool(CreateCard::new(
                self.element_creation_service.clone(),
                self.lexical_json_converter.clone(),
                element,
            ))
            .tool(CreateClozeCard::new(
                self.element_creation_service.clone(),
                self.lexical_json_converter.clone(),
                element,
            ))
            .tool(FinishExtract::new(
                self.learning_asset_scheduling_service.clone(),
                element,
            ))
            .tool(SearchDocuments::new(
                self.ai_client_provider.clone(),
                context.chat_id,
            )))
    }
}

/// An element deleted since the context was captured leaves the tools without one.
async fn existing_element(
    meta_repository: &dyn MetaRepository,
    element_id: Option<ElementId>,
) -> Result<Option<ElementId>, ToolsProviderError> {
    match element_id {
        Some(element_id) if meta_repository.exists(element_id).await? => Ok(Some(element_id)),
        _ => Ok(None),
    }
}
