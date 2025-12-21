use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;
use url::Url;

use crate::api_types::{
    CharacterInput, ExpandCharacterRequest, ExpandWorldviewRequest, GenerateRequest,
};
use crate::db::{begin_glm_request_log, finish_glm_request_log, AppState};
use crate::glm;
use crate::images::{
    ensure_avatar_fallbacks, fallback_background_data_uri, generate_scene_background_base64,
    maybe_attach_generated_avatars, normalize_cogview_size, pick_background_prompt,
};
use crate::prompt::{clean_json, construct_prompt};
use crate::template::{
    convert_lite_to_full, enforce_request_character_consistency, ensure_minimum_game_graph,
    ensure_request_characters_present, fallback_template_lite, normalize_character_ids,
    normalize_template_endings, normalize_template_nodes, sanitize_template_graph,
    MovieTemplateLite,
};
use crate::types::MovieTemplate;

fn glm_api_key() -> Result<String, StatusCode> {
    std::env::var("GLM_API_KEY")
        .or_else(|_| std::env::var("BIGMODEL_API_KEY"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn resolve_glm_api_key(override_key: Option<&str>) -> Result<String, StatusCode> {
    let from_req = override_key.unwrap_or("").trim();
    if !from_req.is_empty() {
        return Ok(from_req.to_string());
    }
    glm_api_key()
}

fn resolve_glm_endpoint(base_url: Option<&str>) -> Result<String, StatusCode> {
    let raw = base_url.unwrap_or("").trim();
    if raw.is_empty() {
        return Ok("https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string());
    }

    if raw.contains("chat/completions") {
        let u = Url::parse(raw).map_err(|_| StatusCode::BAD_REQUEST)?;
        let scheme = u.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(StatusCode::BAD_REQUEST);
        }
        return Ok(u.to_string());
    }

    let mut s = raw.to_string();
    if !s.ends_with('/') {
        s.push('/');
    }
    let base = Url::parse(&s).map_err(|_| StatusCode::BAD_REQUEST)?;
    let scheme = base.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(StatusCode::BAD_REQUEST);
    }
    base.join("chat/completions")
        .map(|u| u.to_string())
        .map_err(|_| StatusCode::BAD_REQUEST)
}

pub(crate) async fn hello() -> &'static str {
    "Hello from Axum!"
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneratePromptResponse {
    prompt: String,
}

pub(crate) async fn generate_prompt(
    Json(payload): Json<GenerateRequest>,
) -> Json<GeneratePromptResponse> {
    Json(GeneratePromptResponse {
        prompt: construct_prompt(&payload),
    })
}

pub(crate) async fn generate(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<GenerateRequest>,
) -> Result<Json<MovieTemplate>, (StatusCode, Json<serde_json::Value>)> {
    let client_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let theme = payload
        .theme
        .as_deref()
        .or(payload.free_input.as_deref())
        .unwrap_or("Unknown Theme");
    println!("Received generate request: {:?}", theme);

    let prompt = construct_prompt(&payload);
    println!("Prompt constructed.");

    let using_override_key = payload
        .api_key
        .as_ref()
        .is_some_and(|k| !k.trim().is_empty());

    let model = if using_override_key {
        payload.model.as_deref().unwrap_or("glm-4.6v-flash")
    } else {
        "glm-4.6v-flash"
    };

    println!("Init GLM Client with 300s timeout...");
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to build client" })),
            )
        })?;

    let mut messages = vec![];
    messages.push(json!({
        "role": "system",
        "content": "You are a professional interactive movie scriptwriter and game designer. You output ONLY valid JSON. You never output markdown code blocks. You strictly follow the provided TypeScript interface definitions."
    }));

    messages.push(json!({
        "role": "user",
        "content": prompt
    }));

    let request_body = json!({
        "model": model,
        "messages": messages,
        "response_format": { "type": "json_object" },
        "temperature": 1,
        "top_p": 0.95,
        "max_tokens": 8192
    });

    println!(
        "Sending request to GLM (Prompt len: {})...",
        request_body["messages"][1]["content"]
            .as_str()
            .unwrap_or("")
            .len()
    );
    let start = std::time::Instant::now();

    let using_override_key = payload
        .api_key
        .as_ref()
        .is_some_and(|k| !k.trim().is_empty());

    let mut payload_json = serde_json::to_value(&payload).unwrap_or(json!({}));
    if let Some(obj) = payload_json.as_object_mut() {
        obj.remove("apiKey");
    }
    let request_id = begin_glm_request_log(
        &state.db,
        &client_ip,
        "/generate",
        payload_json,
        request_body["messages"][1]["content"]
            .as_str()
            .unwrap_or(""),
        using_override_key,
    )
    .await?;

    let endpoint = match resolve_glm_endpoint(payload.base_url.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid baseUrl"),
                Some(response_time_ms),
            )
            .await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid baseUrl", "code": "INVALID_BASE_URL" })),
            ));
        }
    };

    let api_key = match resolve_glm_api_key(payload.api_key.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Missing GLM API Key"),
                Some(response_time_ms),
            )
            .await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Missing GLM API Key" })),
            ));
        }
    };

    let response = match client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("GLM Request failed: {}", e);
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("GLM Request failed"),
                None,
            )
            .await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "GLM Request failed" })),
            ));
        }
    };

    let duration = start.elapsed();
    println!("GLM Request took: {:?}", duration);

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("GLM Error: {}", error_text);
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

        if glm::contains_limit(&error_text) {
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(glm::GLM_LIMIT_FRIENDLY_MESSAGE),
                Some(response_time_ms),
            )
            .await;
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({ "error": glm::GLM_LIMIT_FRIENDLY_MESSAGE })),
            ));
        }

        finish_glm_request_log(
            &state.db,
            request_id,
            "error",
            None,
            Some(&error_text),
            Some(response_time_ms),
        )
        .await;

        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "GLM Error" })),
        ));
    }

    let response_json: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(_) => {
            let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Failed to parse GLM response"),
                Some(response_time_ms),
            )
            .await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to parse GLM response" })),
            ));
        }
    };

    if let Some(usage) = response_json.get("usage") {
        if let Some(tokens) = usage.get("total_tokens") {
            println!("Token Usage: {}", tokens);
        }
    }

    let content = match response_json["choices"][0]["message"]["content"].as_str() {
        Some(c) => c,
        None => {
            let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid GLM response structure"),
                Some(response_time_ms),
            )
            .await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Invalid GLM response" })),
            ));
        }
    };

    println!("GLM Response Content Length: {}", content.len());

    let clean_json_str = clean_json(content);
    let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

    let mut parse_error: Option<String> = None;
    let template_lite: MovieTemplateLite = match serde_json::from_str(&clean_json_str) {
        Ok(t) => {
            println!("JSON deserialization successful. Converting to full template.");
            t
        }
        Err(e) => {
            eprintln!("JSON Error: {}", e);
            parse_error = Some(format!("Parse Error: {}", e));
            println!("Returning fallback template due to JSON error.");
            fallback_template_lite("Error generating story")
        }
    };

    let language_tag = payload.language.as_deref().unwrap_or("zh-CN");
    let mut template = convert_lite_to_full(template_lite, language_tag);
    normalize_character_ids(&mut template);
    normalize_template_nodes(&mut template);
    normalize_template_endings(&mut template);
    
    // Force overwrite characters with frontend input if provided
    if let Some(chars) = &payload.characters {
        if !chars.is_empty() {
             template.characters.clear();
             for c in chars {
                 template.characters.insert(c.name.clone(), crate::types::Character {
                     id: c.name.clone(),
                     name: c.name.clone(),
                     gender: c.gender.clone(),
                     age: 20,
                     role: if c.is_main { "Protagonist".to_string() } else { "Supporting".to_string() },
                     background: c.description.clone(),
                     avatar_path: None,
                 });
             }
        }
    }
    
    ensure_minimum_game_graph(&mut template, language_tag, payload.characters.clone());
    ensure_request_characters_present(&mut template, &payload);
    enforce_request_character_consistency(&mut template, &payload);
    normalize_character_ids(&mut template);
    normalize_template_endings(&mut template);
    sanitize_template_graph(&mut template);

    let size = normalize_cogview_size(payload.size.as_deref());
    let synopsis_for_image = pick_background_prompt(&payload, &template);
    match generate_scene_background_base64(
        &client,
        &synopsis_for_image,
        language_tag,
        &size,
        &api_key,
    )
    .await
    {
        Ok(img) => template.background_image_base64 = Some(img),
        Err(_) => {
            template.background_image_base64 = Some(fallback_background_data_uri(
                &template.title,
                &synopsis_for_image,
            ))
        }
    }

    maybe_attach_generated_avatars(
        &client,
        &mut template,
        payload.characters.as_ref(),
        language_tag,
        &api_key,
    )
    .await;

    ensure_avatar_fallbacks(&mut template, payload.characters.as_ref());

    finish_glm_request_log(
        &state.db,
        request_id,
        "success",
        Some(content),
        parse_error.as_deref(),
        Some(response_time_ms),
    )
    .await;

    Ok(Json(template))
}

#[derive(Serialize)]
struct ExpandWorldviewResponse {
    worldview: String,
}

pub(crate) async fn expand_worldview(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExpandWorldviewRequest>,
) -> impl IntoResponse {
    let client_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let language = req.language.as_deref().unwrap_or("zh-CN");
    let prompt = if let Some(existing) = &req.synopsis {
        format!(
            "Role: Professional Screenwriter / Novelist.\n\
            Task: Expand and refine the following Story Synopsis based on the theme '{}' .\n\
            Existing Synopsis: '{}'\n\
            Language: Output strictly in {}.\n\
            Requirements:
            1. Length: MUST be between 300 and 600 characters (in the target language).
            2. Consistency: STRICTLY PRESERVE all existing characters, relationships, and key plot points mentioned in the input.
            3. Expansion: Add more details to the world setting, atmosphere, and conflict escalation.
            4. Output: Pure text only, no prefixes/suffixes.
            5. Tone: Engaging, cinematic, suspenseful.",
            req.theme, existing, language
        )
    } else {
        format!(
            "Role: Professional Screenwriter / Novelist.
            Task: Write a concise Movie Synopsis (电影简介) for an interactive movie game based on the theme '{}' .
            Language: Output strictly in {}.
            Requirements:
            1. Length: MUST be between 300 and 600 characters (in the target language).
            2. Content: Describe the world setting, main conflict, and atmosphere.
            3. Output: Pure text only, no prefixes/suffixes.
            4. Tone: Engaging, cinematic, suspenseful.",
            req.theme, language
        )
    };

    let using_override_key = req.api_key.as_ref().is_some_and(|k| !k.trim().is_empty());
    let mut payload_json = serde_json::to_value(&req).unwrap_or(json!({}));
    if let Some(obj) = payload_json.as_object_mut() {
        obj.remove("apiKey");
    }

    let request_id = match begin_glm_request_log(
        &state.db,
        &client_ip,
        "/expand/worldview",
        payload_json,
        &prompt,
        using_override_key,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    let start = std::time::Instant::now();
    match glm::call_glm_with_api_key(prompt, false, req.api_key.clone(), req.base_url.clone(), req.model.clone()).await
    {
        Ok(text) => {
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            finish_glm_request_log(
                &state.db,
                request_id,
                "success",
                Some(&text),
                None,
                Some(response_time_ms),
            )
            .await;
            Json(ExpandWorldviewResponse { worldview: text }).into_response()
        }
        Err(e) => {
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            if e == "Invalid baseUrl" {
                finish_glm_request_log(
                    &state.db,
                    request_id,
                    "error",
                    None,
                    Some(&e),
                    Some(response_time_ms),
                )
                .await;
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "Invalid baseUrl", "code": "INVALID_BASE_URL" })),
                )
                    .into_response();
            }

            let status = if e == glm::GLM_LIMIT_FRIENDLY_MESSAGE {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&e),
                Some(response_time_ms),
            )
            .await;
            (status, Json(json!({ "error": e }))).into_response()
        }
    }
}

#[derive(Serialize)]
struct ExpandCharacterResponse {
    characters: Vec<CharacterInput>,
}

pub(crate) async fn expand_character(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExpandCharacterRequest>,
) -> impl IntoResponse {
    let client_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let language = req.language.as_deref().unwrap_or("zh-CN");
    // Use worldview as the synopsis source since frontend sends it in 'worldview' field
    let synopsis_content = if !req.worldview.is_empty() {
        Some(&req.worldview)
    } else {
        req.synopsis.as_ref()
    };

    let prompt = if let Some(synopsis) = synopsis_content {
        format!(
            "你是一名资深电影编剧。

请为一部【{}】电影，基于以下故事大纲，生成一个完整、立体、真实可信的角色设定。

故事大纲：
{}

要求：
1. 数量要求：至少生成 3 个主要角色（根据剧情复杂度可适当增加）。
2. 角色基本信息（姓名、年龄、性别、职业、社会阶层）
   - 性别字段是必填项，禁止为空！必须明确为 '男'、'女' 或 '其他'。
3. 外貌特征（用于电影镜头表现）
4. 性格特质（优点、缺点、矛盾点）
5. 角色的“表层目标”（他/她现在想要什么）
6. 角色的“深层需求”（内心真正缺失的东西）
7. 角色的创伤或过去经历（推动性格形成）
8. 角色在故事中的功能（主角 / 反派 / 配角 / 镜像角色）
9. 角色可能经历的转变弧线（开场 → 结尾）
10. 一句能概括该角色的核心主题句

请避免模板化、脸谱化角色，强调现实逻辑与情感动机。

# 语言要求
输出语言：{}。

# 输出格式
请输出为 JSON 数组，格式如下：
[
  {{
    \"name\": \"角色姓名\",
    \"gender\": \"男\", // 严禁为空！必须是 \"男\" 或 \"女\" 或 \"其他\"
    \"isMain\": true/false,
    \"description\": \"这里包含上述所有详细设定（外貌、性格、目标、创伤等），请组织成一段通顺的文字或使用换行符分隔。\"
  }}
]
注意：必须严格遵守 JSON 格式，不要包含 Markdown 代码块标记。",
            req.theme, synopsis, language
        )
    } else {
        format!(
            "你是一名资深电影编剧。

请为一部【{}】电影，生成一个完整、立体、真实可信的角色设定。

要求：
1. 数量要求：至少生成 3 个主要角色（根据剧情复杂度可适当增加）。
2. 角色基本信息（姓名、年龄、性别、职业、社会阶层）
   - 性别字段是必填项，禁止为空！必须明确为 '男'、'女' 或 '其他'。
3. 外貌特征（用于电影镜头表现）
4. 性格特质（优点、缺点、矛盾点）
5. 角色的“表层目标”（他/她现在想要什么）
6. 角色的“深层需求”（内心真正缺失的东西）
7. 角色的创伤或过去经历（推动性格形成）
8. 角色在故事中的功能（主角 / 反派 / 配角 / 镜像角色）
9. 角色可能经历的转变弧线（开场 → 结尾）
10. 一句能概括该角色的核心主题句

请避免模板化、脸谱化角色，强调现实逻辑与情感动机。

# 语言要求
输出语言：{}。

# 输出格式
请输出为 JSON 数组，格式如下：
[
  {{
    \"name\": \"角色姓名\",
    \"gender\": \"男\", // 严禁为空！必须是 \"男\" 或 \"女\" 或 \"其他\"
    \"isMain\": true/false,
    \"description\": \"这里包含上述所有详细设定（外貌、性格、目标、创伤等），请组织成一段通顺的文字或使用换行符分隔。\"
  }}
]
注意：必须严格遵守 JSON 格式，不要包含 Markdown 代码块标记。",
            req.theme, language
        )
    };

    let using_override_key = req.api_key.as_ref().is_some_and(|k| !k.trim().is_empty());
    let mut payload_json = serde_json::to_value(&req).unwrap_or(json!({}));
    if let Some(obj) = payload_json.as_object_mut() {
        obj.remove("apiKey");
    }

    let request_id = match begin_glm_request_log(
        &state.db,
        &client_ip,
        "/expand/character",
        payload_json,
        &prompt,
        using_override_key,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    let start = std::time::Instant::now();
    let (base_url_arg, model_arg) = if using_override_key {
        (req.base_url.clone(), req.model.clone())
    } else {
        (None, None)
    };

    match glm::call_glm_with_api_key(prompt, true, req.api_key.clone(), base_url_arg, model_arg).await
    {
        Ok(json_str) => {
            let clean = clean_json(&json_str);
            match serde_json::from_str::<Vec<CharacterInput>>(&clean) {
                Ok(chars) => {
                    let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                    finish_glm_request_log(
                        &state.db,
                        request_id,
                        "success",
                        Some(&clean),
                        None,
                        Some(response_time_ms),
                    )
                    .await;
                    Json(ExpandCharacterResponse { characters: chars }).into_response()
                }
                Err(e) => {
                    let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                    finish_glm_request_log(
                        &state.db,
                        request_id,
                        "failed",
                        None,
                        Some(&format!("Parse Error: {}", e)),
                        Some(response_time_ms),
                    )
                    .await;
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Parse Error: {}", e),
                    )
                    .into_response()
                }
            }
        }
        Err(e) => {
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            if e == "Invalid baseUrl" {
                finish_glm_request_log(
                    &state.db,
                    request_id,
                    "failed",
                    None,
                    Some(&e),
                    Some(response_time_ms),
                )
                .await;
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "Invalid baseUrl", "code": "INVALID_BASE_URL" })),
                )
                    .into_response();
            }

            let status = if e == glm::GLM_LIMIT_FRIENDLY_MESSAGE {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some(&e),
                Some(response_time_ms),
            )
            .await;
            (status, Json(json!({ "error": e }))).into_response()
        }
    }
}
