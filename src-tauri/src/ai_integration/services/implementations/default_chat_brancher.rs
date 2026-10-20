use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use injector_derive::ScopeInjectable;
use uuid::Uuid;

use crate::ai_integration::entities::chat::Chat;
use crate::ai_integration::entities::context_snippet::ContextSnippet;
use crate::ai_integration::entities::message::Message;
use crate::ai_integration::repositories::ai_repository::AiRepository;
use crate::ai_integration::services::chat_brancher::{ChatBrancher, ChatBrancherError};

#[derive(ScopeInjectable)]
pub struct DefaultChatBrancher {
    ai_repository: Arc<dyn AiRepository>,
}

#[async_trait]
impl ChatBrancher for DefaultChatBrancher {
    async fn branch_chat(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
    ) -> Result<Chat, ChatBrancherError> {
        let chat = self.ai_repository.get_chat_by_id(chat_id).await?;
        let messages = self
            .ai_repository
            .get_chat_messages_ordered(chat_id)
            .await?;
        let end = messages
            .iter()
            .position(|message| message.id() == message_id)
            .ok_or(ChatBrancherError::MessageNotInChat)?;
        let copied = &messages[..=end];

        let branch = Chat::new(None, format!("{} (branch)", chat.title()));
        self.ai_repository.upsert_chat(&branch).await?;

        // Shifted so the last copied message is dated now, keeping their order and the branch atop the chat list.
        let shift = Utc::now() - copied[end].created_date();
        let mut new_ids = HashMap::with_capacity(copied.len());
        for message in copied {
            let copy = Message::new_unchecked(
                Uuid::new_v4(),
                message.created_date() + shift,
                branch.id(),
                message.content().clone(),
            );
            self.ai_repository.upsert_message(&copy).await?;
            new_ids.insert(message.id(), copy.id());
        }

        for snippet in self
            .ai_repository
            .get_context_snippets_for_chat(chat_id)
            .await?
        {
            let Some(&new_message_id) = new_ids.get(&snippet.message_id()) else {
                continue;
            };
            let copy = ContextSnippet::new(
                None,
                new_message_id,
                snippet.snippet().to_string(),
                snippet.position(),
            );
            self.ai_repository.upsert_context_snippet(&copy).await?;
        }

        Ok(branch)
    }
}
