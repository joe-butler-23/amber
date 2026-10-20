use std::sync::Arc;

use rig::tool::{Tool, ToolContext};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::ai_integration::prompts::CREATE_CLOZE_CARD_DESCRIPTION;
use crate::ai_integration::tools::app_tool::AppTool;
use crate::ai_integration::tools::create_card::{
    CreateCardOutput, card_approval_markdown, card_approved_markdown, card_name,
};
use crate::common::services::lexical_json_converter::{
    LexicalJsonConverter, LexicalJsonConverterError,
};
use crate::elements::dto::create_card_dto::CreateCardDto;
use crate::elements::dto::create_meta_dto::CreateMetaDto;
use crate::elements::services::element_creation_service::{
    ElementCreationError, ElementCreationService,
};
use crate::elements::value_objects::element_id::ElementId;
use crate::elements::value_objects::origin::Origin;

/// What the front shows in place of the hidden part, as the editor's cloze does.
const CLOZE_PLACEHOLDER: &str = "[...]";

#[derive(Deserialize, Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct CreateClozeCardArgs {
    #[schemars(
        description = "The card's text, with exactly one part to hide wrapped in double curly braces, e.g. \"The capital of France is {{Paris}}.\""
    )]
    pub text: String,
}

#[derive(Error, Debug)]
pub enum CreateClozeCardError {
    #[error("The text must wrap exactly one non-empty part to hide in {{{{ and }}}}")]
    InvalidCloze,
    #[error("Failed to create the card")]
    Creation(#[from] ElementCreationError),
    #[error("Failed to convert the card content")]
    Conversion(#[from] LexicalJsonConverterError),
}

/// The text split around its one hidden part.
#[derive(Debug, PartialEq, Eq)]
struct Cloze<'a> {
    before: &'a str,
    hidden: &'a str,
    after: &'a str,
}

impl<'a> Cloze<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let (before, rest) = text.split_once("{{")?;
        let (hidden, after) = rest.split_once("}}")?;
        let stray_braces = [before, hidden, after]
            .iter()
            .any(|part| part.contains("{{") || part.contains("}}"));
        (!stray_braces && !hidden.trim().is_empty()).then_some(Self {
            before,
            hidden,
            after,
        })
    }

    /// The text with the hidden part replaced by `placeholder`.
    fn with(&self, placeholder: &str) -> String {
        format!("{}{placeholder}{}", self.before, self.after)
    }

    /// The front, hiding the part behind the editor's cloze node so the back can reveal it.
    fn front_markdown(&self) -> String {
        self.with(&format!(
            "<mark data-cloze-hidden=\"{}\">{CLOZE_PLACEHOLDER}</mark>",
            escape_html_attribute(self.hidden)
        ))
    }
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub struct CreateClozeCard {
    element_creation_service: Arc<dyn ElementCreationService>,
    lexical_json_converter: Arc<dyn LexicalJsonConverter>,
    parent: Option<ElementId>,
}

impl CreateClozeCard {
    pub fn new(
        element_creation_service: Arc<dyn ElementCreationService>,
        lexical_json_converter: Arc<dyn LexicalJsonConverter>,
        parent: Option<ElementId>,
    ) -> Self {
        Self {
            element_creation_service,
            lexical_json_converter,
            parent,
        }
    }
}

impl Tool for CreateClozeCard {
    const NAME: &'static str = "create_cloze_card";

    type Error = CreateClozeCardError;
    type Args = CreateClozeCardArgs;
    type Output = CreateCardOutput;

    fn description(&self) -> String {
        CREATE_CLOZE_CARD_DESCRIPTION.to_string()
    }

    fn parameters(&self) -> Value {
        serde_json::to_value(schema_for!(CreateClozeCardArgs)).unwrap()
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let cloze = Cloze::parse(&args.text).ok_or(CreateClozeCardError::InvalidCloze)?;

        let front_markdown = cloze.front_markdown();
        let (front, back) = tokio::try_join!(
            self.lexical_json_converter
                .convert_markdown(&front_markdown),
            self.lexical_json_converter.convert_markdown(cloze.hidden),
        )?;

        let card_id = Uuid::new_v4();
        let dto = CreateCardDto {
            id: card_id,
            meta: CreateMetaDto {
                name: card_name(&cloze.with(cloze.hidden)),
                parent: self.parent,
                origin: Origin::Inherited,
            },
            front,
            back,
        };

        self.element_creation_service.create_card(dto).await?;

        Ok(CreateCardOutput { card_id })
    }
}

impl AppTool for CreateClozeCard {
    const REQUIRES_APPROVAL: bool = true;

    /// Malformed text falls back to the raw arguments; the call then fails and tells the model why.
    fn approval_markdown(&self, args: &Self::Args) -> Option<String> {
        let cloze = Cloze::parse(&args.text)?;
        Some(card_approval_markdown(
            self.parent,
            &cloze.with(&format!("**{CLOZE_PLACEHOLDER}**")),
            cloze.hidden,
        ))
    }

    fn approved_markdown(&self, _args: &Self::Args, output: &Self::Output) -> Option<String> {
        Some(card_approved_markdown(output.card_id))
    }
}
