#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::mpsc;
    use std::time::Duration;

    use crate::types::MovieTemplate;
    use crate::types::{AffinityEffect, Choice, MetaInfo, Provenance, StoryNode};
    use serde_json::{from_str, to_string};

    use crate::api_types::GenerateRequest;

    const TEST_TIMEOUT: Duration = Duration::from_secs(10);

    fn run_with_timeout<F>(timeout: Duration, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<std::thread::Result<()>>();

        std::thread::spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let _ = tx.send(res);
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(())) => {}
            Ok(Err(p)) => std::panic::resume_unwind(p),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                panic!("test timed out after {timeout:?}")
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("test thread disconnected")
            }
        }
    }

    #[test]
    fn test_generate_request_deserialize_without_range_fields() {
        run_with_timeout(TEST_TIMEOUT, || {
            let json_data = r#"{
                "mode": "free",
                "freeInput": "下班后被老板叫回去，我很烦",
                "language": "zh-CN"
            }"#;

            let result: Result<GenerateRequest, _> = from_str(json_data);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_generate_request_deserialize_with_legacy_range_fields() {
        run_with_timeout(TEST_TIMEOUT, || {
            let json_data = r#"{
                "mode": "wizard",
                "theme": "职场",
                "synopsis": "测试",
                "minNodes": 5,
                "maxNodes": 15,
                "minEndings": 3,
                "maxEndings": 5,
                "language": "zh-CN"
            }"#;

            let result: Result<GenerateRequest, _> = from_str(json_data);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_choice_serialization_omits_null_affinity_effect() {
        run_with_timeout(TEST_TIMEOUT, || {
            let choice = Choice {
                text: "go".to_string(),
                next_node_id: "1".to_string(),
                affinity_effect: None,
            };

            let json = to_string(&choice).unwrap();
            assert!(!json.contains("affinityEffect"));

            let choice2 = Choice {
                text: "go".to_string(),
                next_node_id: "1".to_string(),
                affinity_effect: Some(AffinityEffect {
                    character_id: "Alice".to_string(),
                    delta: 10,
                }),
            };

            let json2 = to_string(&choice2).unwrap();
            assert!(json2.contains("affinityEffect"));
        });
    }

    #[test]
    fn test_pick_background_prompt_prefers_template_synopsis() {
        run_with_timeout(TEST_TIMEOUT, || {
            let req: GenerateRequest = from_str(
                r#"{
                  "mode": "wizard",
                  "theme": "职场",
                  "synopsis": "REQ",
                  "language": "zh-CN"
                }"#,
            )
            .unwrap();

            let template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "TEMPLATE".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes: HashMap::new(),
                endings: HashMap::new(),
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "u".to_string(),
                    created_at: "t".to_string(),
                },
            };

            let picked = crate::images::pick_background_prompt(&req, &template);
            assert_eq!(picked, "TEMPLATE");
        });
    }

    #[test]
    fn test_fallback_image_data_uris_have_svg_prefix() {
        run_with_timeout(TEST_TIMEOUT, || {
            let bg = crate::images::fallback_background_data_uri("Title", "Synopsis");
            assert!(bg.starts_with("data:image/svg+xml;base64,"));
            let avatar = crate::images::fallback_avatar_data_uri("Alice");
            assert!(avatar.starts_with("data:image/svg+xml;base64,"));
        });
    }

    #[test]
    fn test_deserialize_movie_template() {
        run_with_timeout(TEST_TIMEOUT, || {
            let json_data = r#"{
            "projectId": "12345",
            "title": "Test Movie",
            "version": "1.0.0",
            "owner": "User",
            "meta": {
                "logline": "A test movie",
                "synopsis": "This is a test synopsis",
                "targetRuntimeMinutes": 10,
                "genre": ["Sci-Fi", "Drama"],
                "language": "zh-CN"
            },
            "globalSettings": {
                "resolution": "1920x1080",
                "fps": 24,
                "colorSpace": "Rec.709",
                "audioSampleRate": 48000
            },
            "initialState": { "flags": {}, "variables": {} },
            "nodes": {},
            "characters": {},
            "assets": { "images": [], "audio": [], "models": [] },
            "artifacts": [],
            "iterationLog": [],
            "provenance": { "createdBy": "AI", "createdAt": "2023-10-27" }
        }"#;

            let result: Result<MovieTemplate, _> = from_str(json_data);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_robust_deserialization_synopsis_array() {
        run_with_timeout(TEST_TIMEOUT, || {
            let json_data = r#"{
            "projectId": "12345",
            "title": "Test Movie",
            "version": "1.0.0",
            "owner": "User",
            "meta": {
                "logline": "A test movie",
                "synopsis": ["This is a test synopsis", "Part 2"],
                "targetRuntimeMinutes": 10,
                "genre": ["Sci-Fi", "Drama"],
                "language": "zh-CN"
            },
            "globalSettings": {
                "resolution": "1920x1080",
                "fps": 24,
                "colorSpace": "Rec.709",
                "audioSampleRate": 48000
            },
            "initialState": { "flags": {}, "variables": {} },
            "nodes": {},
            "characters": {},
            "assets": { "images": [], "audio": [], "models": [] },
            "artifacts": [],
            "iterationLog": [],
            "provenance": { "createdBy": "AI", "createdAt": "2023-10-27" }
        }"#;

            let result: Result<MovieTemplate, _> = from_str(json_data);
            assert!(
                result.is_ok(),
                "Should successfully deserialize array as joined string"
            );
            let template = result.unwrap();
            assert_eq!(template.meta.synopsis, "This is a test synopsis\nPart 2");
        });
    }

    #[test]
    fn test_normalize_nodes_key_and_choice_target() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut nodes: HashMap<String, StoryNode> = HashMap::new();

            nodes.insert(
                "node_start".to_string(),
                StoryNode {
                    id: "node_start".to_string(),
                    content: "...".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![Choice {
                        text: "go".to_string(),
                        next_node_id: "node_1".to_string(),
                        affinity_effect: None,
                    }],
                },
            );

            nodes.insert(
                "node_1".to_string(),
                StoryNode {
                    id: "node_1".to_string(),
                    content: "...".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![],
                },
            );

            nodes.insert(
                "n_keep".to_string(),
                StoryNode {
                    id: "n_keep".to_string(),
                    content: "...".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![],
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes,
                endings: HashMap::new(),
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::template::normalize_template_nodes(&mut template);

            assert!(template.nodes.contains_key("start"));
            assert!(template.nodes.contains_key("1"));
            assert!(template.nodes.contains_key("keep"));
            assert!(!template.nodes.contains_key("node_start"));
            assert!(!template.nodes.contains_key("node_1"));
            assert!(!template.nodes.contains_key("n_keep"));

            let start = template.nodes.get("start").unwrap();
            assert_eq!(start.id, "start");
            assert_eq!(start.choices[0].next_node_id, "1");
        });
    }

    #[test]
    fn test_ensure_minimum_game_graph_when_empty() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "".to_string(),
                },
                background_image_base64: None,
                nodes: HashMap::new(),
                endings: HashMap::new(),
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::template::ensure_minimum_game_graph(&mut template, "zh-CN", None);

            assert!(template.nodes.contains_key("start"));
            assert!(!template.nodes.is_empty());
            assert!(template.endings.len() >= 3);
            assert_eq!(template.meta.language, "zh-CN");
        });
    }

    #[test]
    fn test_ensure_minimum_game_graph_uses_request_protagonist_name() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "".to_string(),
                },
                background_image_base64: None,
                nodes: HashMap::new(),
                endings: HashMap::new(),
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            let req_chars = vec![crate::api_types::CharacterInput {
                name: "李雷".to_string(),
                description: "测试主角".to_string(),
                gender: "Male".to_string(),
                is_main: true,
            }];

            crate::template::ensure_minimum_game_graph(&mut template, "zh-CN", Some(req_chars));

            let start = template.nodes.get("start").unwrap();
            let chars = start.characters.as_ref().unwrap();
            assert!(chars.iter().any(|c| c == "李雷"));
            assert!(template.characters.values().any(|c| c.name == "李雷"));
        });
    }

    #[test]
    fn test_normalize_cogview_size_defaults_and_accepts_known_values() {
        run_with_timeout(TEST_TIMEOUT, || {
            assert_eq!(crate::images::normalize_cogview_size(None), "1024x1024");
            assert_eq!(crate::images::normalize_cogview_size(Some("")), "1024x1024");
            assert_eq!(
                crate::images::normalize_cogview_size(Some(" 1152x864 ")),
                "1152x864"
            );
            assert_eq!(
                crate::images::normalize_cogview_size(Some("864x1152")),
                "864x1152"
            );
            assert_eq!(
                crate::images::normalize_cogview_size(Some("1024x1024")),
                "1024x1024"
            );
            assert_eq!(
                crate::images::normalize_cogview_size(Some("999x999")),
                "1024x1024"
            );
        });
    }

    #[test]
    fn test_normalize_endings_key_and_choice_target() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut nodes: HashMap<String, StoryNode> = HashMap::new();

            nodes.insert(
                "n_start".to_string(),
                StoryNode {
                    id: "n_start".to_string(),
                    content: "...".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![Choice {
                        text: "go".to_string(),
                        next_node_id: "bad_end".to_string(),
                        affinity_effect: None,
                    }],
                },
            );

            let mut endings: HashMap<String, crate::types::Ending> = HashMap::new();
            endings.insert(
                "ending_bad".to_string(),
                crate::types::Ending {
                    r#type: "bad".to_string(),
                    description: "bad".to_string(),
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes,
                endings,
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::template::normalize_template_endings(&mut template);

            let start = template.nodes.get("n_start").unwrap();
            assert_eq!(start.choices[0].next_node_id, "ending_bad");
            assert!(template.endings.contains_key("ending_bad"));
        });
    }

    #[test]
    fn test_enforce_request_character_consistency_replaces_player_placeholder() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut nodes: HashMap<String, StoryNode> = HashMap::new();
            nodes.insert(
                "n_start".to_string(),
                StoryNode {
                    id: "n_start".to_string(),
                    content: "...".to_string(),
                    ending_key: None,
                    level: None,
                    characters: Some(vec!["玩家".to_string(), "张三".to_string()]),
                    choices: vec![],
                },
            );

            let mut characters: HashMap<String, crate::types::Character> = HashMap::new();
            characters.insert(
                "c1".to_string(),
                crate::types::Character {
                    id: "c1".to_string(),
                    name: "玩家".to_string(),
                    gender: "Male".to_string(),
                    age: 28,
                    role: "".to_string(),
                    background: "".to_string(),
                    avatar_path: None,
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes,
                endings: HashMap::new(),
                characters,
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            let req = GenerateRequest {
                mode: "wizard".to_string(),
                theme: None,
                synopsis: None,
                genre: None,
                characters: Some(vec![crate::api_types::CharacterInput {
                    name: "张三".to_string(),
                    description: "测试主角".to_string(),
                    gender: "Male".to_string(),
                    is_main: true,
                }]),
                min_nodes: None,
                max_nodes: None,
                min_endings: None,
                max_endings: None,
                free_input: None,
                language: Some("zh-CN".to_string()),
                size: None,
                api_key: None,
                base_url: None,
                model: None,
            };

            crate::template::enforce_character_consistency(&mut template, req.characters.clone());

            let start = template.nodes.get("n_start").unwrap();
            let chars = start.characters.as_ref().unwrap();
            assert_eq!(chars[0], "张三");
            assert!(template.characters.values().any(|c| c.name == "张三"));
        });
    }

    #[test]
    fn test_ensure_request_characters_present_and_avatar_fallback_attaches() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes: HashMap::new(),
                endings: HashMap::new(),
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            template.characters.insert(
                "c_1".to_string(),
                crate::types::Character {
                    id: "c_1".to_string(),
                    name: "SomeoneElse".to_string(),
                    gender: "".to_string(),
                    age: 20,
                    role: "Supporting".to_string(),
                    background: "".to_string(),
                    avatar_path: None,
                },
            );

            let req_chars = vec![crate::api_types::CharacterInput {
                name: "Alice".to_string(),
                description: "Main character".to_string(),
                gender: "Female".to_string(),
                is_main: true,
            }];

            let req = crate::api_types::GenerateRequest {
                mode: "wizard".to_string(),
                theme: None,
                synopsis: None,
                genre: None,
                characters: Some(req_chars.clone()),
                min_nodes: None,
                max_nodes: None,
                min_endings: None,
                max_endings: None,
                free_input: None,
                language: Some("zh-CN".to_string()),
                size: None,
                api_key: None,
                base_url: None,
                model: None,
            };

            crate::template::enforce_character_consistency(&mut template, req.characters.clone());
            assert!(template.characters.values().any(|c| c.name == "Alice"));

            crate::images::ensure_avatar_fallbacks(&mut template, Some(&req_chars));
            let alice = template
                .characters
                .values()
                .find(|c| c.name == "Alice")
                .unwrap();
            assert!(alice
                .avatar_path
                .as_deref()
                .unwrap_or("")
                .starts_with("data:image/"));
        });
    }

    #[test]
    fn test_sanitize_template_graph_breaks_cycle_and_self_reference() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut nodes: HashMap<String, StoryNode> = HashMap::new();

            nodes.insert(
                "n_start".to_string(),
                StoryNode {
                    id: "n_start".to_string(),
                    content: "start".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![Choice {
                        text: "to 02".to_string(),
                        next_node_id: "n_02".to_string(),
                        affinity_effect: None,
                    }],
                },
            );

            nodes.insert(
                "n_02".to_string(),
                StoryNode {
                    id: "n_02".to_string(),
                    content: "two".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![
                        Choice {
                            text: "back".to_string(),
                            next_node_id: "n_start".to_string(),
                            affinity_effect: None,
                        },
                        Choice {
                            text: "self".to_string(),
                            next_node_id: "n_02".to_string(),
                            affinity_effect: None,
                        },
                    ],
                },
            );

            let mut endings: HashMap<String, crate::types::Ending> = HashMap::new();
            endings.insert(
                "ending_neutral".to_string(),
                crate::types::Ending {
                    r#type: "neutral".to_string(),
                    description: "neutral".to_string(),
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes,
                endings,
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::template::sanitize_template_graph(&mut template);

            let n2 = template.nodes.get("n_02").unwrap();
            assert!(n2.choices.iter().all(|c| c.next_node_id != "n_02"));
            assert!(n2.choices.iter().all(|c| c.next_node_id != "n_start"));
            assert!(n2
                .choices
                .iter()
                .any(|c| c.next_node_id == "ending_neutral"));
        });
    }

    #[test]
    fn test_sanitize_template_graph_rewrites_invalid_choice_targets() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut nodes: HashMap<String, StoryNode> = HashMap::new();
            nodes.insert(
                "n_start".to_string(),
                StoryNode {
                    id: "n_start".to_string(),
                    content: "start".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![Choice {
                        text: "go".to_string(),
                        next_node_id: "n_missing".to_string(),
                        affinity_effect: None,
                    }],
                },
            );

            let mut endings: HashMap<String, crate::types::Ending> = HashMap::new();
            endings.insert(
                "ending_neutral".to_string(),
                crate::types::Ending {
                    r#type: "neutral".to_string(),
                    description: "neutral".to_string(),
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes,
                endings,
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::template::sanitize_template_graph(&mut template);
            let start = template.nodes.get("n_start").unwrap();
            assert_eq!(start.choices[0].next_node_id, "ending_neutral");
        });
    }

    #[test]
    fn test_sanitize_template_graph_deduplicates_nodes_and_preserves_ending_key() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut nodes: HashMap<String, StoryNode> = HashMap::new();

            nodes.insert(
                "n_start".to_string(),
                StoryNode {
                    id: "n_start".to_string(),
                    content: "start".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![Choice {
                        text: "go".to_string(),
                        next_node_id: "n_03".to_string(),
                        affinity_effect: None,
                    }],
                },
            );

            nodes.insert(
                "n_02".to_string(),
                StoryNode {
                    id: "n_02".to_string(),
                    content: "dup".to_string(),
                    ending_key: None,
                    level: None,
                    characters: None,
                    choices: vec![Choice {
                        text: "end".to_string(),
                        next_node_id: "ending_good".to_string(),
                        affinity_effect: None,
                    }],
                },
            );

            nodes.insert(
                "n_03".to_string(),
                StoryNode {
                    id: "n_03".to_string(),
                    content: "dup".to_string(),
                    ending_key: Some("ending_good".to_string()),
                    level: None,
                    characters: None,
                    choices: vec![Choice {
                        text: "end".to_string(),
                        next_node_id: "ending_good".to_string(),
                        affinity_effect: None,
                    }],
                },
            );

            let mut endings: HashMap<String, crate::types::Ending> = HashMap::new();
            endings.insert(
                "ending_good".to_string(),
                crate::types::Ending {
                    r#type: "good".to_string(),
                    description: "good".to_string(),
                },
            );
            endings.insert(
                "ending_neutral".to_string(),
                crate::types::Ending {
                    r#type: "neutral".to_string(),
                    description: "neutral".to_string(),
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes,
                endings,
                characters: HashMap::new(),
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::template::sanitize_template_graph(&mut template);

            assert!(!template.nodes.contains_key("n_03"));
            let start = template.nodes.get("n_start").unwrap();
            assert_eq!(start.choices[0].next_node_id, "n_02");

            let owner = template.nodes.get("n_02").unwrap();
            assert_eq!(owner.ending_key.as_deref(), Some("ending_good"));
            assert!(owner.choices.is_empty());
        });
    }

    #[test]
    fn test_attach_avatar_to_template_sets_avatar_path() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut characters: HashMap<String, crate::types::Character> = HashMap::new();
            characters.insert(
                "c_1".to_string(),
                crate::types::Character {
                    id: "c_1".to_string(),
                    name: "Alice".to_string(),
                    gender: "Female".to_string(),
                    age: 20,
                    role: "Protagonist".to_string(),
                    background: "".to_string(),
                    avatar_path: None,
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes: HashMap::new(),
                endings: HashMap::new(),
                characters,
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::images::attach_avatar_to_template(
                &mut template,
                "Alice",
                "data:image/png;base64,AAA".to_string(),
            );

            let c = template.characters.get("c_1").unwrap();
            assert_eq!(c.avatar_path.as_deref(), Some("data:image/png;base64,AAA"));
        });
    }

    #[test]
    fn test_attach_avatar_to_template_does_not_overwrite_existing() {
        run_with_timeout(TEST_TIMEOUT, || {
            let mut characters: HashMap<String, crate::types::Character> = HashMap::new();
            characters.insert(
                "c_1".to_string(),
                crate::types::Character {
                    id: "c_1".to_string(),
                    name: "Alice".to_string(),
                    gender: "Female".to_string(),
                    age: 20,
                    role: "Protagonist".to_string(),
                    background: "".to_string(),
                    avatar_path: Some("data:image/png;base64,OLD".to_string()),
                },
            );

            let mut template = MovieTemplate {
                project_id: "p".to_string(),
                title: "t".to_string(),
                version: "v".to_string(),
                owner: "o".to_string(),
                meta: MetaInfo {
                    logline: "l".to_string(),
                    synopsis: "s".to_string(),
                    target_runtime_minutes: 1,
                    genre: "Drama".to_string(),
                    language: "zh-CN".to_string(),
                },
                background_image_base64: None,
                nodes: HashMap::new(),
                endings: HashMap::new(),
                characters,
                provenance: Provenance {
                    created_by: "c".to_string(),
                    created_at: "a".to_string(),
                },
            };

            crate::images::attach_avatar_to_template(
                &mut template,
                "Alice",
                "data:image/png;base64,NEW".to_string(),
            );

            let c = template.characters.get("c_1").unwrap();
            assert_eq!(c.avatar_path.as_deref(), Some("data:image/png;base64,OLD"));
        });
    }
}
