use std::sync::Arc;

use async_trait::async_trait;
use injector_derive::ScopeInjectable;
use rig::client::AgentClientExt;
use uuid::Uuid;

use crate::ai_integration::entities::message::Message;
use crate::ai_integration::prompts::preamble;
use crate::ai_integration::services::agent_provider::{
    AgentProvider, AgentProviderError, AppAgent,
};
use crate::ai_integration::services::ai_client_provider::AiClientProvider;
use crate::ai_integration::services::tools_provider::ToolsProvider;
use crate::ai_integration::tools::element_reference::element_reference;
use crate::bibliographical_sources::services::bibliographical_source_service::BibliographicalSourceService;
use crate::elements::repositories::meta_repository::MetaRepository;
use crate::elements::value_objects::element_id::ElementId;

const DEFAULT_TEMPERATURE: f64 = 0.5;
const DEFAULT_MAX_TURN: usize = 16;

#[derive(ScopeInjectable)]
pub struct DefaultAgentProvider {
    ai_client_provider: Arc<dyn AiClientProvider>,
    meta_repository: Arc<dyn MetaRepository>,
    bibliographical_source_service: Arc<dyn BibliographicalSourceService>,
    tools_provider: Arc<dyn ToolsProvider>,
}

impl DefaultAgentProvider {
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
}

#[async_trait]
impl AgentProvider for DefaultAgentProvider {
    async fn get_agent(
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
