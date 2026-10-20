use std::sync::Arc;

use rig::tool::{Tool, ToolContext};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::ai_integration::entities::message::Message;
use crate::ai_integration::prompts::FINISH_EXTRACT_DESCRIPTION;
use crate::ai_integration::tools::app_tool::AppTool;
use crate::ai_integration::tools::element_reference::element_reference;
use crate::elements::value_objects::element_id::ElementId;
use crate::study::services::learning_asset_scheduling_service::{
    LearningAssetSchedulingError, LearningAssetSchedulingService,
};

/// Takes nothing; the tool always acts on the extract the chat was opened on.
#[derive(Deserialize, Debug, Clone, Default, Serialize, schemars::JsonSchema)]
pub struct FinishExtractArgs {}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishExtractOutput {
    pub extract_id: Uuid,
}

#[derive(Error, Debug)]
pub enum FinishExtractError {
    #[error("The chat has no open extract")]
    NoExtract,
    #[error("Failed to finish the extract")]
    Scheduling(#[from] LearningAssetSchedulingError),
}

pub struct FinishExtract {
    learning_asset_scheduling_service: Arc<dyn LearningAssetSchedulingService>,
    extract: Option<Uuid>,
}

impl FinishExtract {
    /// `element` is the chat's open element; the tool is only offered when it's an extract.
    pub fn new(
        learning_asset_scheduling_service: Arc<dyn LearningAssetSchedulingService>,
        element: Option<ElementId>,
    ) -> Self {
        Self {
            learning_asset_scheduling_service,
            extract: match element {
                Some(ElementId::Extract(id)) => Some(id),
                _ => None,
            },
        }
    }
}

impl Tool for FinishExtract {
    const NAME: &'static str = "finish_extract";

    type Error = FinishExtractError;
    type Args = FinishExtractArgs;
    type Output = FinishExtractOutput;

    fn description(&self) -> String {
        FINISH_EXTRACT_DESCRIPTION.to_string()
    }

    fn parameters(&self) -> Value {
        serde_json::to_value(schema_for!(FinishExtractArgs)).unwrap()
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        _args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let extract_id = self.extract.ok_or(FinishExtractError::NoExtract)?;

        self.learning_asset_scheduling_service
            .finish(ElementId::Extract(extract_id), None)
            .await?;

        Ok(FinishExtractOutput { extract_id })
    }
}

impl AppTool for FinishExtract {
    const REQUIRES_APPROVAL: bool = true;

    fn is_applicable(&self, _messages: &[Message]) -> bool {
        self.extract.is_some()
    }

    fn approval_markdown(&self, _args: &Self::Args) -> Option<String> {
        Some(match self.extract {
            Some(id) => format!("Finish {}", element_reference(ElementId::Extract(id))),
            None => "Finish the extract; it no longer exists".to_string(),
        })
    }
}
