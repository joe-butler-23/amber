use std::sync::Arc;

use injector_derive::ScopeInjectable;
use rig::agent::Agent;
use rig::client::AgentClientExt;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::ai_integration::approval_hook::ApprovalHook;
use crate::ai_integration::entities::message::Message;
use crate::ai_integration::prompts::preamble;
use crate::ai_integration::services::ai_client_provider::{AiClientProvider, AiClientProviderError};
use crate::ai_integration::services::tools_provider::{ToolsProvider, ToolsProviderError};
use crate::ai_integration::tools::element_reference::element_reference;
use crate::bibliographical_sources::services::bibliographical_source_service::BibliographicalSourceService;
use crate::common::repository_error::RepositoryError;
use crate::elements::repositories::meta_repository::MetaRepository;
use crate::elements::value_objects::element_id::ElementId;

#[derive(Error, Debug)]
pub enum AgentProviderError {
    #[error(transparent)]
    AiClientProvider(#[from] AiClientProviderError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Tools(#[from] ToolsProviderError),
}

/// An agent and the hook that holds back its calls needing approval, to add to each request.
pub struct AppAgent {
    pub agent: Agent,
    pub approval_hook: ApprovalHook,
    /// What its tools were built from, saved with each call awaiting approval to run it later.
    pub tools_context: Value,
}

const DEFAULT_TEMPERATURE: f64 = 0.5;
const DEFAULT_MAX_TURN: usize = 16;

#[derive(ScopeInjectable)]
pub struct AgentProvider {
    ai_client_provider: Arc<AiClientProvider>,
    meta_repository: Arc<dyn MetaRepository>,
    bibliographical_source_service: Arc<dyn BibliographicalSourceService>,
    tools_provider: Arc<dyn ToolsProvider>,
}

impl AgentProvider {
    /// Builds the "**Context:**" section of the preamble: the name of the
    /// element the user is currently viewing, if relevant the bibliographical
    /// source (title + authors) it was derived from. Snippets travel with the
    /// message they were attached to instead.
    async fn build_context(
        &self,
        element_id: Option<ElementId>,
    ) -> Result<Option<String>, AgentProviderError> {
        let mut lines = Vec::new();

        if let Some(element_id) = element_id {
            let meta = self.meta_repository.get_by_id(element_id.id()).await?;
            lines.push(format!(
                "- The user is currently viewing a {} named \"{}\" ({}).",
                element_id.element_name(),
                meta.name,
                element_reference(element_id)
            ));

            if let Some(bibliographical_source_id) = meta.bibliographical_source_id {
                let source = self
                    .bibliographical_source_service
                    .get_bibliographical_source(bibliographical_source_id)
                    .await?
                    .bibliographical_source;

                let origin = match source.authors {
                    Some(authors) => format!("\"{}\" by {authors}", source.title),
                    None => format!("\"{}\"", source.title),
                };
                lines.push(format!("- It originates from {origin}."));
            }
        }

        if lines.is_empty() {
            return Ok(None);
        }

        Ok(Some(lines.join("\n")))
    }

    pub async fn get_agent(
        &self,
        chat_id: Uuid,
        messages: &[Message],
        element_id: Option<ElementId>,
    ) -> Result<AppAgent, AgentProviderError> {
        let client = self.ai_client_provider.get_client().await?;
        let completion_model_name = self.ai_client_provider.get_completion_model_name().await?;
        let context = self.build_context(element_id).await?;

        let builder = client
            .agent(&completion_model_name)
            .temperature(DEFAULT_TEMPERATURE)
            .name("Amber Tutor")
            .default_max_turns(DEFAULT_MAX_TURN)
            .preamble(preamble(context.as_deref()).as_str());

        let tools_context = self.tools_provider.capture_context(chat_id, element_id);
        let (tool_server, approval_hook) = self
            .tools_provider
            .get_tools(&tools_context)
            .await?
            .into_agent_tools(messages);

        Ok(AppAgent {
            agent: builder.tool_server_handle(tool_server).build(),
            approval_hook,
            tools_context,
        })
    }
}
