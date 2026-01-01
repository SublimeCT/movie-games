use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;

use crate::api_types::CharacterInput;
use crate::types::{self, MovieTemplate};

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
    nodes: Option<HashMap<String, StoryNodeLiteOrString>>,
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
#[serde(untagged)]
enum StoryNodeLiteOrString {
    Node(StoryNodeLite),
    // Fallback for cases where node is just a string content
    String(String),
    // Fallback for empty object or null
    Empty {},
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoryNodeLite {
    id: Option<String>,
    node_id: Option<String>,
    #[serde(alias = "text")]
    content: Option<String>, // Support 'text' as alias for 'content'
    ending_key: Option<String>,
    level: Option<u32>,
    characters: Option<Vec<String>>,
    choices: Option<Vec<ChoiceLite>>,
}

fn convert_node_lite(key: String, lite: StoryNodeLite) -> types::StoryNode {
    types::StoryNode {
        id: lite.id.or(lite.node_id).unwrap_or(key),
        content: lite.content.unwrap_or_else(|| "...".to_string()),
        ending_key: lite.ending_key,
        level: lite.level,
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
    #[serde(default)]
    affinity_effect: Option<types::AffinityEffect>,
}

impl From<ChoiceLite> for types::Choice {
    fn from(lite: ChoiceLite) -> Self {
        types::Choice {
            text: lite.text.unwrap_or_else(|| "Continue".to_string()),
            next_node_id: lite.next_node_id.unwrap_or_else(|| "END".to_string()),
            affinity_effect: lite.affinity_effect,
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
            .filter_map(|(k, v)| match v {
                StoryNodeLiteOrString::Node(node) => Some((k.clone(), convert_node_lite(k, node))),
                StoryNodeLiteOrString::String(s) => {
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some((
                            k.clone(),
                            types::StoryNode {
                                id: k,
                                content: s,
                                ending_key: None,
                                level: None,
                                characters: None,
                                choices: Vec::new(),
                            },
                        ))
                    }
                }
                StoryNodeLiteOrString::Empty {} => None,
            })
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

pub(crate) fn normalize_character_ids(template: &mut MovieTemplate) {
    // Rebuild characters map with name as key (as per user requirement)
    let mut new_characters: HashMap<String, types::Character> = HashMap::new();

    for (k, c) in template.characters.iter() {
        let key = if !c.name.is_empty() {
            c.name.clone()
        } else if !c.id.is_empty() {
            c.id.clone()
        } else {
            k.clone()
        };

        let mut char = c.clone();
        char.id = key.clone();
        new_characters.insert(key, char);
    }

    template.characters = new_characters;
}

pub(crate) fn normalize_template_nodes(template: &mut MovieTemplate) {
    if template.nodes.is_empty() {
        return;
    }

    // Direct pass-through of nodes if they are already in the correct format.
    // User explicitly requested: "禁止任何数据结构的转换"
    // However, we still need to ensure consistency, e.g. "start" node id if missing.
    // But we should NOT add "n_" prefix.

    let mut mapping: HashMap<String, String> = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();

    // Sort keys to ensure deterministic order (optional but good for consistency)
    let mut keys: Vec<String> = template.nodes.keys().cloned().collect();
    keys.sort();

    for old_key in keys {
        // Ensure "start" is "start", and other keys are kept as is (or sanitized if needed)
        // User required: "nodes 中的 key 改为纯数字(例如 1 / 2 / 3), 开始节点的 key 固定为 start"
        // If GLM returns "start", keep it "start".
        // If GLM returns "1", keep it "1".
        // If GLM returns "n_1", strip "n_" if user insists on pure numbers, OR just keep as is?
        // User said: "禁止使用之前的 n_xxx 这种类型的节点 key" AND "禁止做任何数据结构的转换"
        // This is a bit contradictory if GLM returns "n_1".
        // BUT, if prompt says "key 改为纯数字", then GLM should return "1".
        // If GLM obeys, we just need to NOT add "n_".

        let new_key = if old_key == "start" || old_key == "n_start" {
            "start".to_string()
        } else {
            // If key is "n_1", maybe we should strip "n_" to comply with "pure numbers"?
            // Let's be safe and strip known prefixes if they exist, but otherwise keep as is.
            if let Some(stripped) = old_key.strip_prefix("n_") {
                stripped.to_string()
            } else if let Some(stripped) = old_key.strip_prefix("node_") {
                stripped.to_string()
            } else {
                old_key.clone()
            }
        };

        // Handle duplicates if stripping prefixes causes collisions (unlikely but possible)
        let mut final_key = new_key.clone();
        let mut i = 2usize;
        while used.contains_key(&final_key) {
            final_key = format!("{}_{}", new_key, i);
            i += 1;
        }

        used.insert(final_key.clone(), 1);
        if final_key != old_key {
            mapping.insert(old_key.clone(), final_key);
        }
    }

    if mapping.is_empty() {
        for (k, node) in template.nodes.iter_mut() {
            if node.id.is_empty() {
                node.id = k.clone();
            }
        }
        return;
    }

    let old_nodes = std::mem::take(&mut template.nodes);
    let mut new_nodes: HashMap<String, types::StoryNode> = HashMap::new();

    for (old_key, mut node) in old_nodes {
        let new_key = mapping
            .get(&old_key)
            .cloned()
            .unwrap_or_else(|| old_key.clone());

        if node.id.is_empty() || node.id == old_key {
            node.id = new_key.clone();
        }

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
    if let Some(pos) = keys.iter().position(|k| k == "start") {
        keys.remove(pos);
        keys.insert(0, "start".to_string());
    } else if let Some(pos) = keys.iter().position(|k| k == "n_start") {
        // Just in case it wasn't normalized yet, but sanitize is usually after normalize
        keys.remove(pos);
        keys.insert(0, "n_start".to_string());
    }

    for node_id in keys.iter() {
        if node_id == "start" || node_id == "n_start" {
            signature_owner.insert(node_id.clone(), "".to_string());
            continue;
        }

        let Some(node) = template.nodes.get(node_id) else {
            continue;
        };

        let text = node.content.trim().to_string();
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
    if let Some(pos) = node_ids.iter().position(|k| k == "start") {
        node_ids.remove(pos);
        node_ids.insert(0, "start".to_string());
    } else if let Some(pos) = node_ids.iter().position(|k| k == "n_start") {
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

pub(crate) fn sanitize_affinity_effects(template: &mut MovieTemplate) {
    if template.nodes.is_empty() {
        return;
    }

    let mut id_to_name: HashMap<String, String> = HashMap::new();
    for c in template.characters.values() {
        let id = c.id.trim();
        let name = c.name.trim();
        if !id.is_empty() && !name.is_empty() {
            id_to_name.insert(id.to_string(), name.to_string());
        }
    }

    let protagonist = pick_protagonist_name(&template.characters);

    for node in template.nodes.values_mut() {
        let allowed: HashMap<String, ()> = node
            .characters
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|raw| {
                let v = raw.trim().to_string();
                if v.is_empty() {
                    return None;
                }
                let resolved = id_to_name.get(&v).cloned().unwrap_or(v);
                Some((resolved, ()))
            })
            .collect();

        for c in node.choices.iter_mut() {
            let Some(effect) = c.affinity_effect.as_mut() else {
                continue;
            };

            effect.delta = effect.delta.clamp(-20, 20);

            let raw = effect.character_id.trim().to_string();
            if raw.is_empty() {
                c.affinity_effect = None;
                continue;
            }

            let resolved = id_to_name.get(&raw).cloned().unwrap_or(raw);
            effect.character_id = resolved.clone();

            if let Some(p) = protagonist.as_ref() {
                if p == &resolved {
                    c.affinity_effect = None;
                    continue;
                }
            }

            if !allowed.contains_key(&resolved) {
                c.affinity_effect = None;
            }
        }
    }
}

fn pick_protagonist_name(chars: &HashMap<String, types::Character>) -> Option<String> {
    if chars.is_empty() {
        return None;
    }

    let mut best: Option<(i32, String)> = None;

    for (k, c) in chars.iter() {
        let key = k.to_lowercase();
        let name = c.name.trim();
        if name.is_empty() {
            continue;
        }

        let mut score: i32 = 0;
        let role = c.role.to_lowercase();
        let name_l = name.to_lowercase();

        if key.contains("player") || key.contains("protagonist") || key.contains("main") {
            score += 5;
        }
        if role.contains("protagonist") || role.contains("player") || role.contains("main") {
            score += 6;
        }
        if name == "我" || name.contains("主角") {
            score += 7;
        }
        if name_l.contains("protagonist") || name_l.contains("player") {
            score += 4;
        }

        match best.as_ref() {
            Some((best_score, _)) if *best_score >= score => {}
            _ => {
                best = Some((score, name.to_string()));
            }
        }
    }

    best.map(|(_, name)| name)
}

pub(crate) fn enforce_character_consistency(
    template: &mut MovieTemplate,
    req_characters: Option<Vec<CharacterInput>>,
) {
    let Some(chars) = req_characters else {
        return;
    };

    let mut allowed: Vec<String> = Vec::new();
    let mut out: HashMap<String, types::Character> = HashMap::new();

    for input_char in chars {
        let name = input_char.name.trim().to_string();
        if name.is_empty() {
            continue;
        }

        allowed.push(name.clone());

        out.insert(
            name.clone(),
            types::Character {
                id: name.clone(),
                name: name.clone(),
                gender: input_char.gender,
                age: 0,
                role: input_char.description,
                background: String::new(),
                avatar_path: None,
            },
        );
    }

    let allowed_set: std::collections::HashSet<String> = allowed.into_iter().collect();

    for node in template.nodes.values_mut() {
        let Some(list) = node.characters.as_mut() else {
            continue;
        };

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        list.retain(|raw| {
            let n = raw.trim().to_string();
            if n.is_empty() {
                return false;
            }
            if !allowed_set.contains(&n) {
                return false;
            }
            if seen.contains(&n) {
                return false;
            }
            seen.insert(n);
            true
        });

        if list.is_empty() {
            node.characters = None;
        }
    }

    template.characters = out;
}

#[allow(dead_code)]
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

    enforce_character_consistency(template, req_characters);

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

    if template.nodes.is_empty()
        || (!template.nodes.contains_key("start") && !template.nodes.contains_key("n_start"))
    {
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

        // Use "start" as user requested, not "n_start"
        template.nodes.insert(
            "start".to_string(),
            types::StoryNode {
                id: "start".to_string(),
                content: "下班的电梯门合上那一刻，我手机震了一下。屏幕上只有一句：‘回来一趟。’我盯着那行字，胃里像被拧了一把。回去，就等于把自己再塞回那间会议室；不回去，明天的账只会更难算。门外的风很冷，我却更怕那句没有语气的命令。".to_string(),
                ending_key: None,
                level: Some(1),
                characters: Some(vec![protagonist_name.clone()]),
                choices: vec![
                    types::Choice {
                        text: "回去，当面把话说清楚".to_string(),
                        next_node_id: "confront".to_string(), // use pure id
                        affinity_effect: None,
                    },
                    types::Choice {
                        text: "装作没看见，先离开".to_string(),
                        next_node_id: "escape".to_string(), // use pure id
                        affinity_effect: None,
                    },
                ],
            },
        );

        template.nodes.insert(
            "confront".to_string(),
            types::StoryNode {
                id: "confront".to_string(),
                content: "我转身往回走，每一步都像踩在自己心虚上。进门前我深吸一口气：今天的锅我不背，但我也不躲。对方的目光压过来时，我把手心里的汗收住，先把边界摆出来。".to_string(),
                ending_key: None,
                level: Some(2),
                characters: Some(vec![protagonist_name.clone()]),
                choices: vec![
                    types::Choice {
                        text: "坚持边界".to_string(),
                        next_node_id: "ending_good".to_string(),
                        affinity_effect: None,
                    },
                    types::Choice {
                        text: "妥协退让".to_string(),
                        next_node_id: "ending_bad".to_string(),
                        affinity_effect: None,
                    },
                ],
            },
        );

        template.nodes.insert(
            "escape".to_string(),
            types::StoryNode {
                id: "escape".to_string(),
                content: "我关掉屏幕，快步走向地铁站。心里那个声音一直在吵：‘躲得过初一，躲不过十五。’但至少今晚，这几个小时是我的。".to_string(),
                ending_key: None,
                level: Some(2),
                characters: Some(vec![protagonist_name.clone()]),
                choices: vec![
                    types::Choice {
                        text: "回家休息".to_string(),
                        next_node_id: "ending_neutral".to_string(),
                        affinity_effect: None,
                    },
                ],
            },
        );
    }
}

// REMOVED: enforce_request_character_consistency and ensure_request_characters_present
// because they were unused and user requested cleanup.
