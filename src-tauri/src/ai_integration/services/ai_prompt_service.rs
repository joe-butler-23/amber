use std::sync::Arc;

use chrono::Utc;
use injector_derive::ScopeInjectable;
use thiserror::Error;
use uuid::Uuid;

use crate::ai_integration::entities::ai_prompt::AiPrompt;
use crate::ai_integration::repositories::ai_prompt_repository::AiPromptRepository;
use crate::common::repository_error::RepositoryError;

#[derive(Error, Debug)]
pub enum AiPromptError {
    #[error("A prompt needs a name and text")]
    Empty,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Manages the user's prompt library.
#[derive(ScopeInjectable)]
pub struct AiPromptService {
    ai_prompt_repository: Arc<dyn AiPromptRepository>,
}

impl AiPromptService {
    pub async fn list_prompts(&self) -> Result<Vec<AiPrompt>, AiPromptError> {
        Ok(self.ai_prompt_repository.get_all().await?)
    }

    /// Trims the name and content, refusing either being blank.
    pub async fn create_prompt(
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

    /// Trims the name and content, refusing either being blank.
    pub async fn update_prompt(
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

    pub async fn delete_prompt(&self, id: Uuid) -> Result<(), AiPromptError> {
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
