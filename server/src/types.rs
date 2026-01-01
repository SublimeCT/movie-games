use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(s) => Ok(s),
        StringOrVec::Vec(v) => Ok(v.join("\n")),
    }
}

fn deserialize_string_or_vec_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_or_vec(deserializer)
}

fn deserialize_option_vec_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionVecOrString {
        Vec(Vec<String>),
        String(String),
    }

    let opt: Option<OptionVecOrString> = Option::deserialize(deserializer)?;
    match opt {
        Some(OptionVecOrString::Vec(v)) => Ok(Some(v)),
        Some(OptionVecOrString::String(s)) => Ok(Some(vec![s])),
        None => Ok(None),
    }
}

fn deserialize_characters<'de, D>(deserializer: D) -> Result<HashMap<String, Character>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MapOrVec {
        Map(HashMap<String, Character>),
        Vec(Vec<Character>),
    }

    match MapOrVec::deserialize(deserializer)? {
        MapOrVec::Map(m) => Ok(m),
        MapOrVec::Vec(v) => {
            let mut m = HashMap::new();
            for c in v {
                let key = if !c.id.is_empty() {
                    c.id.clone()
                } else if !c.name.is_empty() {
                    c.name.clone()
                } else {
                    format!("char_{}", m.len())
                };
                m.insert(key, c);
            }
            Ok(m)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MovieTemplate {
    pub project_id: String,
    pub title: String,
    pub version: String,
    pub owner: String,
    pub meta: MetaInfo,
    #[serde(default)]
    pub background_image_base64: Option<String>,
    #[serde(default)]
    pub nodes: HashMap<String, StoryNode>,
    #[serde(default)]
    pub endings: HashMap<String, Ending>,
    #[serde(default, deserialize_with = "deserialize_characters")]
    pub characters: HashMap<String, Character>,
    #[serde(default)]
    pub provenance: Provenance,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetaInfo {
    #[serde(deserialize_with = "deserialize_string_or_vec")]
    pub logline: String,
    #[serde(deserialize_with = "deserialize_string_or_vec")]
    pub synopsis: String,
    pub target_runtime_minutes: u32,
    #[serde(deserialize_with = "deserialize_string_or_vec_to_string")]
    pub genre: String,
    pub language: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub age: u32,
    pub role: String,
    pub background: String,
    pub avatar_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoryNode {
    pub id: String, // Renamed from node_id
    pub content: String,
    #[serde(default)]
    pub ending_key: Option<String>,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_option_vec_or_string")]
    pub characters: Option<Vec<String>>,
    #[serde(default)]
    pub choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AffinityEffect {
    pub character_id: String,
    pub delta: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub text: String,
    pub next_node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affinity_effect: Option<AffinityEffect>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ending {
    pub r#type: String, // 'good' | 'neutral' | 'bad'
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub created_by: String,
    pub created_at: String,
}
