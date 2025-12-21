use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;

use crate::api_types::{CharacterInput, GenerateRequest};
use crate::types::{self, Character, MovieTemplate};

fn simple_hash_u32(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

fn deserialize_option_string_or_vec<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionStringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    let opt: Option<OptionStringOrVec> = Option::deserialize(deserializer)?;
    match opt {
        Some(OptionStringOrVec::String(s)) => Ok(Some(s)),
        Some(OptionStringOrVec::Vec(v)) => Ok(Some(v.join("\n"))),
        None => Ok(None),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MovieTemplateLite {
    title: Option<String>,
    meta: Option<MetaInfoLite>,
    nodes: Option<HashMap<String, StoryNodeLite>>,
    characters: Option<HashMap<String, CharacterLite>>,
    endings: Option<HashMap<String, types::Ending>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaInfoLite {
    logline: Option<String>,
    synopsis: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_string_or_vec")]
    genre: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterLite {
    id: Option<String>,
    name: Option<String>,
    gender: Option<String>,
    age: Option<Value>,
    role: Option<String>,
    background: Option<String>,
    avatar_path: Option<String>,
    description: Option<String>,
}

impl From<CharacterLite> for types::Character {
    fn from(lite: CharacterLite) -> Self {
        types::Character {
            id: lite.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: lite.name.unwrap_or_else(|| "Unknown".to_string()),
            gender: lite.gender.unwrap_or_else(|| "Unknown".to_string()),
            age: lite
                .age
                .and_then(|v| v.as_u64().map(|n| n as u32))
                .unwrap_or(0),
            role: lite.role.unwrap_or_default(),
            background: lite.background.or(lite.description).unwrap_or_default(),
            avatar_path: lite.avatar_path,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoryNodeLite {
    id: Option<String>,
    node_id: Option<String>,
    content: Option<NodeContentLite>,
    ending_key: Option<String>,
    characters: Option<Vec<String>>,
    choices: Option<Vec<ChoiceLite>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeContentLite {
    text: Option<String>,
    notes: Option<String>,
}

fn convert_node_lite(key: String, lite: StoryNodeLite) -> types::StoryNode {
    types::StoryNode {
        id: lite.id.or(lite.node_id).unwrap_or(key),
        content: types::NodeContent {
            text: lite
                .content
                .as_ref()
                .and_then(|c| c.text.clone())
                .unwrap_or_else(|| "...".to_string()),
            notes: lite.content.as_ref().and_then(|c| c.notes.clone()),
        },
        ending_key: lite.ending_key,
        characters: lite.characters,
        choices: lite
            .choices
            .map(|choices| choices.into_iter().map(|c| c.into()).collect())
            .unwrap_or_default(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChoiceLite {
    text: Option<String>,
    next_node_id: Option<String>,
}

impl From<ChoiceLite> for types::Choice {
    fn from(lite: ChoiceLite) -> Self {
        types::Choice {
            text: lite.text.unwrap_or_else(|| "Continue".to_string()),
            next_node_id: lite.next_node_id.unwrap_or_else(|| "END".to_string()),
        }
    }
}

pub(crate) fn convert_lite_to_full(lite: MovieTemplateLite, language: &str) -> MovieTemplate {
    MovieTemplate {
        project_id: uuid::Uuid::new_v4().to_string(),
        title: lite.title.unwrap_or_else(|| "Untitled Project".to_string()),
        version: "1.0.0".to_string(),
        owner: "User".to_string(),
        meta: types::MetaInfo {
            logline: lite
                .meta
                .as_ref()
                .and_then(|m| m.logline.clone())
                .unwrap_or_default(),
            synopsis: lite
                .meta
                .as_ref()
                .and_then(|m| m.synopsis.clone())
                .unwrap_or_default(),
            target_runtime_minutes: 0,
            genre: lite
                .meta
                .as_ref()
                .and_then(|m| m.genre.clone())
                .unwrap_or_default(),
            language: language.to_string(),
        },
        background_image_base64: None,
        nodes: lite
            .nodes
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k.clone(), convert_node_lite(k, v)))
            .collect(),
        characters: lite
            .characters
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect(),
        endings: lite.endings.unwrap_or_default(),
        provenance: Default::default(),
    }
}

pub(crate) fn fallback_template_lite(title: &str) -> MovieTemplateLite {
    MovieTemplateLite {
        title: Some(title.to_string()),
        meta: None,
        nodes: None,
        characters: None,
        endings: None,
    }
}

pub(crate) fn normalize_character_ids(template: &mut MovieTemplate) {
    for (k, c) in template.characters.iter_mut() {
        c.id = k.clone();
    }
}

pub(crate) fn normalize_template_nodes(template: &mut MovieTemplate) {
    if template.nodes.is_empty() {
        return;
    }

    let mut mapping: HashMap<String, String> = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();

    for old_key in template.nodes.keys() {
        let base = if old_key == "start" {
            "n_start".to_string()
        } else if old_key.starts_with("n_") {
            old_key.clone()
        } else if old_key.starts_with("node_") {
            format!("n_{}", &old_key[5..])
        } else {
            format!("n_{}", old_key)
        };

        let mut new_key = base.clone();
        let mut i = 2usize;
        while used.contains_key(&new_key) {
            new_key = format!("{}_{}", base, i);
            i += 1;
        }

        used.insert(new_key.clone(), 1);
        mapping.insert(old_key.clone(), new_key);
    }

    let old_nodes = std::mem::take(&mut template.nodes);
    let mut new_nodes: HashMap<String, types::StoryNode> = HashMap::new();

    for (old_key, mut node) in old_nodes {
        let new_key = mapping
            .get(&old_key)
            .cloned()
            .unwrap_or_else(|| old_key.clone());

        node.id = new_key.clone();

        for c in node.choices.iter_mut() {
            if let Some(mapped) = mapping.get(&c.next_node_id) {
                c.next_node_id = mapped.clone();
            }
        }

        new_nodes.insert(new_key, node);
    }

    template.nodes = new_nodes;
}

pub(crate) fn normalize_template_endings(template: &mut MovieTemplate) {
    if template.endings.is_empty() {
        return;
    }

    let canonicalize_key = |k: &str| -> Option<&'static str> {
        match k.trim() {
            "ending_good" | "good_end" | "end_good" | "good" | "GOOD" => Some("ending_good"),
            "ending_neutral" | "neutral_end" | "end_neutral" | "neutral" | "NEUTRAL" => {
                Some("ending_neutral")
            }
            "ending_bad" | "bad_end" | "end_bad" | "bad" | "BAD" => Some("ending_bad"),
            _ => None,
        }
    };

    let mut moved: Vec<(String, String)> = Vec::new();
    for k in template.endings.keys() {
        if let Some(canonical) = canonicalize_key(k) {
            if canonical != k {
                moved.push((k.clone(), canonical.to_string()));
            }
        }
    }

    for (from, to) in moved {
        if template.endings.contains_key(&to) {
            template.endings.remove(&from);
            continue;
        }
        if let Some(v) = template.endings.remove(&from) {
            template.endings.insert(to, v);
        }
    }

    for node in template.nodes.values_mut() {
        for choice in node.choices.iter_mut() {
            if let Some(canonical) = canonicalize_key(&choice.next_node_id) {
                choice.next_node_id = canonical.to_string();
            }
        }
    }

    if template.endings.len() > 5 {
        let mut keep: HashMap<String, types::Ending> = HashMap::new();
        for k in ["ending_good", "ending_neutral", "ending_bad"] {
            if let Some(v) = template.endings.get(k).cloned() {
                keep.insert(k.to_string(), v);
            }
        }

        if keep.len() < 5 {
            for (k, v) in template.endings.iter() {
                if keep.len() >= 5 {
                    break;
                }
                if keep.contains_key(k) {
                    continue;
                }
                keep.insert(k.clone(), v.clone());
            }
        }

        template.endings = keep;
    }
}

pub(crate) fn sanitize_template_graph(template: &mut MovieTemplate) {
    if template.nodes.is_empty() {
        return;
    }

    let ending_neutral_key = if template.endings.contains_key("ending_neutral") {
        "ending_neutral".to_string()
    } else if template.endings.contains_key("ending_bad") {
        "ending_bad".to_string()
    } else if template.endings.contains_key("ending_good") {
        "ending_good".to_string()
    } else {
        "END".to_string()
    };

    let mut signature_owner: HashMap<String, String> = HashMap::new();
    let mut redirect: HashMap<String, String> = HashMap::new();

    let mut keys: Vec<String> = template.nodes.keys().cloned().collect();
    keys.sort();
    if let Some(pos) = keys.iter().position(|k| k == "n_start") {
        keys.remove(pos);
        keys.insert(0, "n_start".to_string());
    }

    for node_id in keys.iter() {
        let Some(node) = template.nodes.get(node_id) else {
            continue;
        };

        let text = node.content.text.trim().to_string();
        let mut cparts: Vec<String> = node
            .choices
            .iter()
            .map(|c| format!("{}→{}", c.text.trim(), c.next_node_id.trim()))
            .collect();
        cparts.sort();
        let signature = format!("{}||{}", text, cparts.join("|"));

        if let Some(owner) = signature_owner.get(&signature) {
            if owner != node_id {
                redirect.insert(node_id.clone(), owner.clone());
            }
        } else {
            signature_owner.insert(signature, node_id.clone());
        }
    }

    if !redirect.is_empty() {
        for node in template.nodes.values_mut() {
            for choice in node.choices.iter_mut() {
                if let Some(to) = redirect.get(&choice.next_node_id) {
                    choice.next_node_id = to.clone();
                }
            }
        }

        for (from, to) in redirect.iter() {
            let from_ending_key = template.nodes.get(from).and_then(|n| n.ending_key.clone());

            if let Some(k) = from_ending_key {
                if let Some(to_node) = template.nodes.get_mut(to) {
                    if to_node.ending_key.is_none() {
                        to_node.ending_key = Some(k);
                    }
                }
            }
        }

        for k in redirect.keys() {
            template.nodes.remove(k);
        }
    }

    let mut state: HashMap<String, u8> = HashMap::new();
    let mut node_ids: Vec<String> = template.nodes.keys().cloned().collect();
    node_ids.sort();
    if let Some(pos) = node_ids.iter().position(|k| k == "n_start") {
        node_ids.remove(pos);
        node_ids.insert(0, "n_start".to_string());
    }

    fn dfs(
        cur: &str,
        template: &mut MovieTemplate,
        state: &mut HashMap<String, u8>,
        ending_fallback: &str,
    ) {
        state.insert(cur.to_string(), 1);

        let outgoing: Vec<String> = template
            .nodes
            .get(cur)
            .map(|n| n.choices.iter().map(|c| c.next_node_id.clone()).collect())
            .unwrap_or_default();

        for next in outgoing {
            if next == cur {
                if let Some(n) = template.nodes.get_mut(cur) {
                    for c in n.choices.iter_mut() {
                        if c.next_node_id == cur {
                            c.next_node_id = ending_fallback.to_string();
                        }
                    }
                }
                continue;
            }

            if !template.nodes.contains_key(&next) {
                continue;
            }

            let next_state = *state.get(&next).unwrap_or(&0);
            if next_state == 1 {
                if let Some(n) = template.nodes.get_mut(cur) {
                    for c in n.choices.iter_mut() {
                        if c.next_node_id == next {
                            c.next_node_id = ending_fallback.to_string();
                        }
                    }
                }
                continue;
            }

            if next_state == 0 {
                dfs(&next, template, state, ending_fallback);
            }
        }

        state.insert(cur.to_string(), 2);
    }

    for id in node_ids {
        if *state.get(&id).unwrap_or(&0) == 0 {
            dfs(&id, template, &mut state, &ending_neutral_key);
        }
    }

    let ending_keys: HashMap<String, ()> =
        template.endings.keys().map(|k| (k.clone(), ())).collect();
    let node_keys: HashMap<String, ()> = template.nodes.keys().map(|k| (k.clone(), ())).collect();

    let ending_fallback = if ending_keys.contains_key(&ending_neutral_key) {
        ending_neutral_key.clone()
    } else {
        template
            .endings
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "END".to_string())
    };

    for node in template.nodes.values_mut() {
        for choice in node.choices.iter_mut() {
            let to = choice.next_node_id.trim();
            if to.is_empty() {
                choice.next_node_id = ending_fallback.clone();
                continue;
            }

            if to == "END" {
                continue;
            }

            if node_keys.contains_key(to) {
                continue;
            }

            if ending_keys.contains_key(to) {
                continue;
            }

            choice.next_node_id = ending_fallback.clone();
        }
    }

    for node in template.nodes.values_mut() {
        if let Some(ending_key) = node.ending_key.as_ref() {
            if ending_keys.contains_key(ending_key) {
                node.choices.clear();
            }
        }
    }

    for node in template.nodes.values_mut() {
        if !node.choices.is_empty() {
            continue;
        }

        let valid = node
            .ending_key
            .as_ref()
            .is_some_and(|k| ending_keys.contains_key(k));

        if valid {
            continue;
        }

        if ending_keys.contains_key(&ending_neutral_key) {
            node.ending_key = Some(ending_neutral_key.clone());
        }
    }
}

pub(crate) fn ensure_minimum_game_graph(
    template: &mut MovieTemplate,
    language_tag: &str,
    req_characters: Option<Vec<CharacterInput>>,
) {
    if template.meta.language.is_empty() {
        template.meta.language = language_tag.to_string();
    }

    let (protagonist_name, protagonist_gender) = req_characters
        .as_ref()
        .and_then(|cs| cs.iter().find(|c| c.is_main).or_else(|| cs.first()))
        .map(|c| (c.name.clone(), c.gender.clone()))
        .unwrap_or_else(|| ("主角".to_string(), "男".to_string()));

    if template.endings.is_empty() {
        template.endings.insert(
            "ending_good".to_string(),
            types::Ending {
                r#type: "good".to_string(),
                description: "我扛住了压力，也守住了边界。".to_string(),
            },
        );
        template.endings.insert(
            "ending_neutral".to_string(),
            types::Ending {
                r#type: "neutral".to_string(),
                description: "我暂时逃开了，但问题没消失。".to_string(),
            },
        );
        template.endings.insert(
            "ending_bad".to_string(),
            types::Ending {
                r#type: "bad".to_string(),
                description: "我把事情拖烂了，明天更难受。".to_string(),
            },
        );
    }

    if template.nodes.is_empty() || !template.nodes.contains_key("n_start") {
        let protagonist_id = "c_player".to_string();
        template
            .characters
            .entry(protagonist_id.clone())
            .or_insert(types::Character {
                id: protagonist_id.clone(),
                name: protagonist_name.clone(),
                gender: protagonist_gender.clone(),
                age: 28,
                role: "员工".to_string(),
                background: "下班时被突然的消息绊住。".to_string(),
                avatar_path: None,
            });

        template.nodes.insert(
            "n_start".to_string(),
            types::StoryNode {
                id: "n_start".to_string(),
                content: types::NodeContent {
                    text: "下班的电梯门合上那一刻，我手机震了一下。屏幕上只有一句：‘回来一趟。’我盯着那行字，胃里像被拧了一把。回去，就等于把自己再塞回那间会议室；不回去，明天的账只会更难算。门外的风很冷，我却更怕那句没有语气的命令。".to_string(),
                    notes: None,
                },
                ending_key: None,
                characters: Some(vec![protagonist_name.clone()]),
                choices: vec![
                    types::Choice {
                        text: "回去，当面把话说清楚".to_string(),
                        next_node_id: "n_confront".to_string(),
                    },
                    types::Choice {
                        text: "装作没看见，先离开".to_string(),
                        next_node_id: "n_escape".to_string(),
                    },
                ],
            },
        );

        template.nodes.insert(
            "n_confront".to_string(),
            types::StoryNode {
                id: "n_confront".to_string(),
                content: types::NodeContent {
                    text: "我转身往回走，每一步都像踩在自己心虚上。进门前我深吸一口气：今天的锅我不背，但我也不躲。对方的目光压过来时，我把手心里的汗收住，先把边界摆出来。".to_string(),
                    notes: None,
                },
                ending_key: None,
                characters: Some(vec![protagonist_name.clone()]),
                choices: vec![
                    types::Choice {
                        text: "坚持边界".to_string(),
                        next_node_id: "ending_good".to_string(),
                    },
                    types::Choice {
                        text: "退一步求稳".to_string(),
                        next_node_id: "ending_neutral".to_string(),
                    },
                ],
            },
        );

        template.nodes.insert(
            "n_escape".to_string(),
            types::StoryNode {
                id: "n_escape".to_string(),
                content: types::NodeContent {
                    text: "我把手机塞进兜里，假装没听见。地铁的轰鸣把我脑子里那句‘回来’冲得更响。我知道自己在拖，但我现在只想把今天结束掉。可越走越快，我越清楚：明天只会更糟。".to_string(),
                    notes: None,
                },
                ending_key: None,
                characters: Some(vec![protagonist_name.clone()]),
                choices: vec![
                    types::Choice {
                        text: "继续逃".to_string(),
                        next_node_id: "ending_bad".to_string(),
                    },
                    types::Choice {
                        text: "回头补救".to_string(),
                        next_node_id: "ending_neutral".to_string(),
                    },
                ],
            },
        );
    }
}

pub(crate) fn enforce_request_character_consistency(
    template: &mut MovieTemplate,
    req: &GenerateRequest,
) {
    let Some(req_chars) = req.characters.as_ref() else {
        return;
    };

    let protagonist = req_chars
        .iter()
        .find(|c| c.is_main)
        .or_else(|| req_chars.first());
    let Some(protagonist) = protagonist else {
        return;
    };

    let canonical_name = protagonist.name.trim().to_string();
    if canonical_name.is_empty() {
        return;
    }

    let canonical_gender = protagonist.gender.trim().to_string();
    let placeholders = [
        "玩家",
        "主角",
        "我",
        "player",
        "Player",
        "protagonist",
        "Protagonist",
    ];

    for node in template.nodes.values_mut() {
        if let Some(chars) = node.characters.as_mut() {
            for name in chars.iter_mut() {
                if placeholders.iter().any(|p| p == name) {
                    *name = canonical_name.clone();
                }
            }
        }
    }

    for c in template.characters.values_mut() {
        if placeholders.iter().any(|p| p == &c.name) {
            c.name = canonical_name.clone();
            if !canonical_gender.is_empty() {
                c.gender = canonical_gender.clone();
            }
        }
    }
}

pub(crate) fn ensure_request_characters_present(
    template: &mut MovieTemplate,
    req: &GenerateRequest,
) {
    let Some(req_chars) = req.characters.as_ref() else {
        return;
    };

    let mut idx = 0usize;
    for rc in req_chars {
        let name = rc.name.trim();
        if name.is_empty() {
            continue;
        }

        let exists = template.characters.values().any(|c| c.name.trim() == name);
        if exists {
            for c in template.characters.values_mut() {
                if c.name.trim() == name {
                    if c.gender.trim().is_empty() {
                        let g = rc.gender.trim();
                        if !g.is_empty() {
                            c.gender = g.to_string();
                        }
                    }
                }
            }
            continue;
        }

        let key = if rc.is_main {
            "player_protagonist".to_string()
        } else {
            idx += 1;
            format!("c_req_{:02}", idx)
        };

        if template.characters.contains_key(&key) {
            idx += 1;
        }

        let key = if template.characters.contains_key(&key) {
            format!("c_req_{}", simple_hash_u32(&format!("{}::{}", name, idx)))
        } else {
            key
        };

        let g = rc.gender.trim();
        let gender = if g.is_empty() {
            "Unknown".to_string()
        } else {
            g.to_string()
        };

        template.characters.insert(
            key.clone(),
            Character {
                id: key,
                name: name.to_string(),
                gender,
                age: 0,
                role: if rc.is_main {
                    "Protagonist".to_string()
                } else {
                    "Supporting".to_string()
                },
                background: rc.description.trim().to_string(),
                avatar_path: None,
            },
        );
    }
}
