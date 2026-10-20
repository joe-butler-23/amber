use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCardDto {
    pub id: Uuid,
    pub front: Option<String>,
    pub back: Option<String>,
}
