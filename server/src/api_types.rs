use crate::types::MovieTemplate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerateResponse {
    pub(crate) id: Uuid,
    pub(crate) template: MovieTemplate,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShareRequest {
    pub(crate) id: Uuid,
    pub(crate) shared: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordsListRequest {
    pub(crate) ids: Vec<Uuid>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateTemplateRequest {
    pub(crate) id: Uuid,
    pub(crate) template: MovieTemplate,
    #[serde(default)]
    pub(crate) source: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteTemplateRequest {
    pub(crate) id: Uuid,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportTemplateRequest {
    pub(crate) template: MovieTemplate,
    #[serde(default)]
    pub(crate) theme: Option<String>,
    #[serde(default)]
    pub(crate) synopsis: Option<String>,
    #[serde(default)]
    pub(crate) genre: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) characters: Option<Vec<CharacterInput>>,
    #[serde(default)]
    pub(crate) language: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerateRequest {
    pub(crate) mode: String,
    pub(crate) theme: Option<String>,
    pub(crate) synopsis: Option<String>,
    pub(crate) genre: Option<Vec<String>>,
    pub(crate) characters: Option<Vec<CharacterInput>>,
    #[serde(default)]
    pub(crate) min_nodes: Option<u32>,
    #[serde(default)]
    pub(crate) max_nodes: Option<u32>,
    #[serde(default)]
    pub(crate) min_endings: Option<u32>,
    #[serde(default)]
    pub(crate) max_endings: Option<u32>,
    pub(crate) free_input: Option<String>,
    pub(crate) language: Option<String>,
    #[serde(default)]
    pub(crate) size: Option<String>,
    #[serde(default)]
    pub(crate) api_key: Option<String>,
    #[serde(default)]
    pub(crate) base_url: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub(crate) struct CharacterInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) gender: String,
    #[serde(rename = "isMain")]
    pub(crate) is_main: bool,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExpandWorldviewRequest {
    pub(crate) theme: String,
    pub(crate) synopsis: Option<String>,
    #[serde(default)]
    pub(crate) genre: Option<Vec<String>>,
    pub(crate) language: Option<String>,
    pub(crate) api_key: Option<String>,
    #[serde(default)]
    pub(crate) base_url: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExpandCharacterRequest {
    pub(crate) theme: String,
    pub(crate) worldview: String,
    pub(crate) synopsis: Option<String>,
    pub(crate) existing_characters: Vec<CharacterInput>,
    #[serde(default)]
    pub(crate) genre: Option<Vec<String>>,
    pub(crate) language: Option<String>,
    pub(crate) api_key: Option<String>,
    #[serde(default)]
    pub(crate) base_url: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
}
