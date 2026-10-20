use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use injector_derive::ScopeInjectable;
use uuid::Uuid;

use crate::ai_integration::entities::ai_prompt::AiPrompt;
use crate::ai_integration::repositories::ai_prompt_repository::AiPromptRepository;
use crate::ai_integration::services::ai_prompt_service::{AiPromptError, AiPromptService};

#[derive(ScopeInjectable)]
pub struct DefaultAiPromptService {
    ai_prompt_repository: Arc<dyn AiPromptRepository>,
}

#[async_trait]
impl AiPromptService for DefaultAiPromptService {
    async fn list_prompts(&self) -> Result<Vec<AiPrompt>, AiPromptError> {
        Ok(self.ai_prompt_repository.get_all().await?)
    }

    async fn create_prompt(
        &self,
        name: String,
        content: String,
    ) -> Result<AiPrompt, AiPromptError> {
        let (name, content) = trimmed(&name, &content)?;
        let now = Utc::now();
        let prompt = AiPrompt {
            id: Uuid::new_v4(),
            created_at: now,
            modified_at: now,
            name,
            content,
        };
        self.ai_prompt_repository.create(&prompt).await?;
        Ok(prompt)
    }

    async fn update_prompt(
        &self,
        id: Uuid,
        name: String,
        content: String,
    ) -> Result<AiPrompt, AiPromptError> {
        let (name, content) = trimmed(&name, &content)?;
        let existing = self.ai_prompt_repository.get_by_id(id).await?;
        let prompt = AiPrompt {
            name,
            content,
            ..existing
        };
        self.ai_prompt_repository.update(&prompt).await?;
        Ok(prompt)
    }

    async fn delete_prompt(&self, id: Uuid) -> Result<(), AiPromptError> {
        Ok(self.ai_prompt_repository.delete(id).await?)
    }
}

fn trimmed(name: &str, content: &str) -> Result<(String, String), AiPromptError> {
    let (name, content) = (name.trim(), content.trim());
    if name.is_empty() || content.is_empty() {
        return Err(AiPromptError::Empty);
    }
    Ok((name.to_string(), content.to_string()))
}
