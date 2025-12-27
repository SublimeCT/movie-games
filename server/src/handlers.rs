use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use std::net::SocketAddr;
use url::Url;
use uuid::Uuid;

use crate::api_types::{
    CharacterInput, ExpandCharacterRequest, ExpandWorldviewRequest, GenerateRequest,
};
use crate::db::{begin_glm_request_log, finish_glm_request_log, AppState, DbError};
use crate::glm;
use crate::images::{
    ensure_avatar_fallbacks, fallback_background_data_uri, generate_scene_background_base64,
    maybe_attach_generated_avatars, normalize_cogview_size, pick_background_prompt,
};
use crate::prompt::{clean_json, construct_prompt};
use crate::template::{
    convert_lite_to_full, normalize_character_ids, normalize_template_endings,
    normalize_template_nodes, sanitize_template_graph, MovieTemplateLite,
};

// ===== 统一响应格式 =====

// 成功时 code = "0"
pub const CODE_SUCCESS: &str = "0";
// 通用错误
// pub const CODE_ERROR: &str = "1";
// API 限流 / 请求过多
pub const CODE_TOO_MANY_REQUESTS: &str = "TOO_MANY_REQUESTS";
// 参数错误
pub const CODE_BAD_REQUEST: &str = "BAD_REQUEST";
// 内部错误
pub const CODE_INTERNAL_ERROR: &str = "INTERNAL_ERROR";
// 无效的 baseUrl
pub const CODE_INVALID_BASE_URL: &str = "INVALID_BASE_URL";

/// 统一 API 响应格式
#[derive(Serialize)]
pub(crate) struct ApiResponse<T> {
    pub(crate) code: String,
    pub(crate) msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub(crate) fn success(data: T) -> Self {
        Self {
            code: CODE_SUCCESS.to_string(),
            msg: "success".to_string(),
            data: Some(data),
        }
    }

    #[allow(dead_code)]
    fn error(code: impl Into<String>, msg: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            code: code.into(),
            msg: msg.into(),
            data: None,
        }
    }

    #[allow(dead_code)]
    fn error_with_data(code: impl Into<String>, msg: impl Into<String>, data: T) -> ApiResponse<T> {
        ApiResponse {
            code: code.into(),
            msg: msg.into(),
            data: Some(data),
        }
    }
}

fn success_response<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse::success(data))
}

fn error_response(code: impl Into<String>, msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<()>>) {
    let code_str = code.into();
    let status = match code_str.as_str() {
        CODE_TOO_MANY_REQUESTS => StatusCode::TOO_MANY_REQUESTS,
        CODE_BAD_REQUEST | CODE_INVALID_BASE_URL => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ApiResponse {
        code: code_str,
        msg: msg.into(),
        data: None,
    }))
}

fn db_error_response(e: DbError) -> (StatusCode, Json<ApiResponse<()>>) {
    error_response(e.code(), e.message())
}

fn rate_limit_response(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<()>>) {
    error_response(CODE_TOO_MANY_REQUESTS, msg)
}

struct GlmRequestGuard {
    db: PgPool,
    request_id: Uuid,
    consumed: bool,
}

impl GlmRequestGuard {
    fn new(db: PgPool, request_id: Uuid) -> Self {
        Self {
            db,
            request_id,
            consumed: false,
        }
    }

    fn consume(mut self) {
        self.consumed = true;
    }
}

impl Drop for GlmRequestGuard {
    fn drop(&mut self) {
        if !self.consumed {
            let db = self.db.clone();
            let id = self.request_id;
            // Spawn a task to update status to cancel
            tokio::spawn(async move {
                finish_glm_request_log(
                    &db,
                    id,
                    "cancel",
                    None,
                    Some("Client disconnected or request cancelled"),
                    None,
                )
                .await;
            });
        }
    }
}

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

pub(crate) async fn generate_prompt(
    Json(payload): Json<GenerateRequest>,
) -> Json<ApiResponse<String>> {
    success_response(construct_prompt(&payload))
}

pub(crate) async fn generate(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<GenerateRequest>,
) -> Result<Response, Response> {
    let client_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());
    
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

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

    println!("Init GLM Client with 240s timeout...");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(240))
        .build()
        .map_err(|e| error_response(CODE_INTERNAL_ERROR, e.to_string()).into_response())?;

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
        user_agent,
        "/generate",
        payload_json,
        request_body["messages"][1]["content"]
            .as_str()
            .unwrap_or(""),
        using_override_key,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let guard = GlmRequestGuard::new(state.db.clone(), request_id);

    let endpoint = match resolve_glm_endpoint(payload.base_url.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid baseUrl"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INVALID_BASE_URL, "Invalid baseUrl").into_response());
        }
    };

    let api_key = match resolve_glm_api_key(payload.api_key.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Missing GLM API Key"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response("API_KEY_REQUIRED", "API Key is required. Please configure your own API Key in settings.").into_response());
        }
    };

    let response = match client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("GLM Request failed: {}", e);
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("GLM Request failed"),
                None,
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "GLM Request failed").into_response());
        }
    };

    let duration = start.elapsed();
    println!("GLM Request took: {:?}", duration);

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("GLM Error: {}", error_text);
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

        // Check for GLM error code 1305 (rate limit)
        if glm::is_rate_limit_error(&error_text) {
            let error_message = if let Some(code) = glm::extract_glm_error_code(&error_text) {
                format!("GLM API 返回错误码 {}: {}", code, error_text)
            } else {
                format!("{}", error_text)
            };

            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&error_text),
                Some(response_time_ms),
            )
            .await;
            return Err(rate_limit_response(error_message).into_response());
        }

        // Fallback: check for "limit" keyword in error text
        if glm::contains_limit(&error_text) {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&error_text),
                Some(response_time_ms),
            )
            .await;
            return Err(rate_limit_response(&error_text).into_response());
        }

        guard.consume();
        finish_glm_request_log(
            &state.db,
            request_id,
            "error",
            None,
            Some(&error_text),
            Some(response_time_ms),
        )
        .await;

        return Err(error_response(CODE_INTERNAL_ERROR, error_text).into_response());
    }

    let text_response = response
        .text()
        .await
        .map_err(|e| {
             // Cannot use guard here as we are returning Err, but we can't consume guard and return err easily without scope
             // Actually we can just log failure and return
             // We'll let the guard drop handle the "cancel" status if read fails, or we can manually log failure.
             // But guard.consume() takes ownership.
             // It is better to let guard handle "cancel" if read fails, or log it as internal error.
             error_response(CODE_INTERNAL_ERROR, format!("Failed to read response body: {}", e)).into_response()
        })?;

    // Try to parse as generic JSON first to check for "error" field
    // (GLM sometimes returns 200 OK with "error" in body)
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text_response) {
        if json_value.get("error").is_some() {
            println!("GLM returned 200 OK but with error body: {}", text_response);
            let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

            if glm::is_rate_limit_error(&text_response) {
                let error_message = if let Some(code) = glm::extract_glm_error_code(&text_response) {
                    format!("GLM API 返回错误码 {}: {}", code, text_response)
                } else {
                    text_response.clone()
                };

                guard.consume();
                finish_glm_request_log(
                    &state.db,
                    request_id,
                    "error",
                    None,
                    Some(&text_response),
                    Some(response_time_ms),
                )
                .await;
                return Err(rate_limit_response(error_message).into_response());
            }

            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&text_response),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, text_response).into_response());
        }
    }

    let response_json: serde_json::Value = match serde_json::from_str(&text_response) {
        Ok(v) => v,
        Err(e) => {
            let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                Some(&text_response), // Log the raw text that failed parsing
                Some(&format!("Failed to parse GLM response JSON: {}", e)),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "Failed to parse GLM response").into_response());
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
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid GLM response structure"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "Invalid GLM response structure").into_response());
        }
    };

    println!("GLM Response Content Length: {}", content.len());

    let clean_json_str = clean_json(content);
    let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

    let template_lite: MovieTemplateLite = match serde_json::from_str(&clean_json_str) {
        Ok(t) => {
            println!("JSON deserialization successful. Converting to full template.");
            t
        }
        Err(e) => {
            eprintln!("JSON Error: {}", e);
            let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                Some(content),
                Some(&format!("JSON Parse Error: {}", e)),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, format!("JSON Parse Error: {}", e)).into_response());
        }
    };

    let language_tag = payload.language.as_deref().unwrap_or("zh-CN");
    let mut template = convert_lite_to_full(template_lite, language_tag);
    normalize_character_ids(&mut template);
    normalize_template_nodes(&mut template);
    normalize_template_endings(&mut template);

    // Only ensure minimum graph if GLM returned nothing - never overwrite GLM's data
    // ensure_minimum_game_graph call removed to prevent write-dead data injection

    // NO character modifications - preserve GLM's original output
    // ensure_request_characters_present(&mut template, &payload);
    
    // User insisted: "Must return character info passed by frontend exactly as is"
    crate::template::enforce_character_consistency(&mut template, payload.characters.clone());

    normalize_character_ids(&mut template);
    normalize_template_endings(&mut template);
    sanitize_template_graph(&mut template);

    // Image generation logic
    let should_generate_images = if using_override_key {
        let standard_url = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
        let input_url = payload.base_url.as_deref().unwrap_or("").trim();
        input_url.is_empty() || input_url == standard_url
    } else {
        true
    };

    if should_generate_images {
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
    } else {
         template.background_image_base64 = Some(fallback_background_data_uri(
            &template.title,
            &template.meta.synopsis,
        ));
    }

    ensure_avatar_fallbacks(&mut template, payload.characters.as_ref());

    guard.consume();
    finish_glm_request_log(
        &state.db,
        request_id,
        "success",
        Some(content),
        None,
        Some(response_time_ms),
    )
    .await;

    Ok(success_response(template).into_response())
}

pub(crate) async fn expand_worldview(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExpandWorldviewRequest>,
) -> Result<Response, Response> {
    let client_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());
    
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

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

    // Initialize Client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(240))
        .build()
        .map_err(|e| error_response(CODE_INTERNAL_ERROR, e.to_string()).into_response())?;

    let request_id = begin_glm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/expand/worldview",
        payload_json,
        &prompt,
        using_override_key,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let guard = GlmRequestGuard::new(state.db.clone(), request_id);

    let endpoint = match resolve_glm_endpoint(req.base_url.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let start = std::time::Instant::now();
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid baseUrl"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INVALID_BASE_URL, "Invalid baseUrl").into_response());
        }
    };

    let api_key = match resolve_glm_api_key(req.api_key.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let start = std::time::Instant::now();
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Missing GLM API Key"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response("API_KEY_REQUIRED", "API Key is required").into_response());
        }
    };

    let model = if using_override_key {
        req.model.as_deref().unwrap_or("glm-4.6v-flash")
    } else {
        "glm-4.6v-flash"
    };

    let messages = vec![
        json!({
            "role": "system",
            "content": "You are a professional interactive movie scriptwriter and game designer."
        }),
        json!({
            "role": "user",
            "content": prompt
        })
    ];

    let request_body = json!({
        "model": model,
        "messages": messages,
        // expand_worldview does NOT force JSON object in original call (json_mode: false)
        // "response_format": { "type": "json_object" }, 
        "temperature": 1,
        "top_p": 0.95,
        "max_tokens": 4096 // Adjusted reasonable limit for text expansion
    });

    let start = std::time::Instant::now();
    let response = match client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("GLM Request failed: {}", e);
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("GLM Request failed"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "GLM Request failed").into_response());
        }
    };

    let duration = start.elapsed();
    let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("GLM Error: {}", error_text);

        if glm::is_rate_limit_error(&error_text) {
            let error_message = if let Some(code) = glm::extract_glm_error_code(&error_text) {
                format!("GLM API 返回错误码 {}: {}", code, error_text)
            } else {
                format!("{}", error_text)
            };

            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&error_text),
                Some(response_time_ms),
            )
            .await;
            return Err(rate_limit_response(error_message).into_response());
        }

        if glm::contains_limit(&error_text) {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&error_text),
                Some(response_time_ms),
            )
            .await;
            return Err(rate_limit_response(&error_text).into_response());
        }

        guard.consume();
        finish_glm_request_log(
            &state.db,
            request_id,
            "error",
            None,
            Some(&error_text),
            Some(response_time_ms),
        )
        .await;
        return Err(error_response(CODE_INTERNAL_ERROR, error_text).into_response());
    }

    let text_response = response
        .text()
        .await
        .map_err(|e| {
             error_response(CODE_INTERNAL_ERROR, format!("Failed to read response body: {}", e)).into_response()
        })?;

    if text_response.trim().is_empty() {
        eprintln!("GLM returned empty response body");
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
        guard.consume();
        finish_glm_request_log(
            &state.db,
            request_id,
            "failed",
            Some(""),
            Some("GLM returned empty response body"),
            Some(response_time_ms),
        )
        .await;
        return Err(error_response(CODE_INTERNAL_ERROR, "GLM returned empty response body").into_response());
    }

    // Check for 200 OK error
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text_response) {
        if json_value.get("error").is_some() {
            println!("GLM returned 200 OK but with error body: {}", text_response);
            
            if glm::is_rate_limit_error(&text_response) {
                let error_message = if let Some(code) = glm::extract_glm_error_code(&text_response) {
                    format!("GLM API 返回错误码 {}: {}", code, text_response)
                } else {
                    text_response.clone()
                };

                guard.consume();
                finish_glm_request_log(
                    &state.db,
                    request_id,
                    "error",
                    None,
                    Some(&text_response),
                    Some(response_time_ms),
                )
                .await;
                return Err(rate_limit_response(error_message).into_response());
            }

            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&text_response),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, text_response).into_response());
        }
    }

    // Extract content from chat response
    let response_json: serde_json::Value = match serde_json::from_str(&text_response) {
        Ok(v) => v,
        Err(e) => {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                Some(&text_response),
                Some(&format!("Failed to parse GLM response JSON: {}", e)),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "Failed to parse GLM response").into_response());
        }
    };

    let content = match response_json["choices"][0]["message"]["content"].as_str() {
        Some(c) => c.to_string(),
        None => {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid GLM response structure"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "Invalid GLM response structure").into_response());
        }
    };

    guard.consume();
    finish_glm_request_log(
        &state.db,
        request_id,
        "success",
        Some(&content),
        None,
        Some(response_time_ms),
    )
    .await;

    Ok(success_response(content).into_response())
}

pub(crate) async fn expand_character(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExpandCharacterRequest>,
) -> Result<Response, Response> {
    let client_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());
    
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

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

    // Initialize Client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(240))
        .build()
        .map_err(|e| error_response(CODE_INTERNAL_ERROR, e.to_string()).into_response())?;

    let request_id = begin_glm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/expand/character",
        payload_json,
        &prompt,
        using_override_key,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let guard = GlmRequestGuard::new(state.db.clone(), request_id);

    let endpoint = match resolve_glm_endpoint(req.base_url.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let start = std::time::Instant::now();
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid baseUrl"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INVALID_BASE_URL, "Invalid baseUrl").into_response());
        }
    };

    let api_key = match resolve_glm_api_key(req.api_key.as_deref()) {
        Ok(v) => v,
        Err(_) => {
            let start = std::time::Instant::now();
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Missing GLM API Key"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response("API_KEY_REQUIRED", "API Key is required").into_response());
        }
    };

    let model = if using_override_key {
        req.model.as_deref().unwrap_or("glm-4.6v-flash")
    } else {
        "glm-4.6v-flash"
    };

    let messages = vec![
        json!({
            "role": "system",
            "content": "You are a professional interactive movie scriptwriter and game designer. Output strictly valid JSON."
        }),
        json!({
            "role": "user",
            "content": prompt
        })
    ];

    let request_body = json!({
        "model": model,
        "messages": messages,
        "response_format": { "type": "json_object" }, // Force JSON for character expansion
        "temperature": 1,
        "top_p": 0.95,
        "max_tokens": 8192
    });

    let start = std::time::Instant::now();
    let response = match client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("GLM Request failed: {}", e);
            let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("GLM Request failed"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "GLM Request failed").into_response());
        }
    };

    let duration = start.elapsed();
    let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("GLM Error: {}", error_text);

        if glm::is_rate_limit_error(&error_text) {
            let error_message = if let Some(code) = glm::extract_glm_error_code(&error_text) {
                format!("GLM API 返回错误码 {}: {}", code, error_text)
            } else {
                format!("{}", error_text)
            };

            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&error_text),
                Some(response_time_ms),
            )
            .await;
            return Err(rate_limit_response(error_message).into_response());
        }

        if glm::contains_limit(&error_text) {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&error_text),
                Some(response_time_ms),
            )
            .await;
            return Err(rate_limit_response(&error_text).into_response());
        }

        guard.consume();
        finish_glm_request_log(
            &state.db,
            request_id,
            "error",
            None,
            Some(&error_text),
            Some(response_time_ms),
        )
        .await;
        return Err(error_response(CODE_INTERNAL_ERROR, error_text).into_response());
    }

    let text_response = response
        .text()
        .await
        .map_err(|e| {
             error_response(CODE_INTERNAL_ERROR, format!("Failed to read response body: {}", e)).into_response()
        })?;

    if text_response.trim().is_empty() {
        eprintln!("GLM returned empty response body");
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
        guard.consume();
        finish_glm_request_log(
            &state.db,
            request_id,
            "failed",
            Some(""),
            Some("GLM returned empty response body"),
            Some(response_time_ms),
        )
        .await;
        return Err(error_response(CODE_INTERNAL_ERROR, "GLM returned empty response body").into_response());
    }

    // Check for 200 OK error
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text_response) {
        if json_value.get("error").is_some() {
            println!("GLM returned 200 OK but with error body: {}", text_response);
            
            if glm::is_rate_limit_error(&text_response) {
                let error_message = if let Some(code) = glm::extract_glm_error_code(&text_response) {
                    format!("GLM API 返回错误码 {}: {}", code, text_response)
                } else {
                    text_response.clone()
                };

                guard.consume();
                finish_glm_request_log(
                    &state.db,
                    request_id,
                    "error",
                    None,
                    Some(&text_response),
                    Some(response_time_ms),
                )
                .await;
                return Err(rate_limit_response(error_message).into_response());
            }

            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "error",
                None,
                Some(&text_response),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, text_response).into_response());
        }
    }

    // Extract content from chat response
    let response_json: serde_json::Value = match serde_json::from_str(&text_response) {
        Ok(v) => v,
        Err(e) => {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                Some(&text_response),
                Some(&format!("Failed to parse GLM response JSON: {}", e)),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "Failed to parse GLM response").into_response());
        }
    };

    let content = match response_json["choices"][0]["message"]["content"].as_str() {
        Some(c) => c,
        None => {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                None,
                Some("Invalid GLM response structure"),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, "Invalid GLM response structure").into_response());
        }
    };

    let clean = clean_json(content);
    match serde_json::from_str::<Vec<CharacterInput>>(&clean) {
        Ok(chars) => {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "success",
                Some(&clean),
                None,
                Some(response_time_ms),
            )
            .await;
            Ok(success_response(chars).into_response())
        }
        Err(e) => {
            guard.consume();
            finish_glm_request_log(
                &state.db,
                request_id,
                "failed",
                Some(&clean),
                Some(&format!("Parse Error: {}", e)),
                Some(response_time_ms),
            )
            .await;
            Err(error_response(CODE_INTERNAL_ERROR, format!("Parse Error: {}", e)).into_response())
        }
    }
}
