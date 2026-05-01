use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use url::Url;
use uuid::Uuid;

use crate::api_types::{
    CharacterInput, DeleteTemplateRequest, ExpandCharacterRequest,
    ExpandWorldviewRequest, GenerateRequest, GenerateResponse, ImportResponse,
    ImportTemplateRequest, RecordsListRequest, ShareRequest, UpdateTemplateRequest,
};
use crate::db::{
    begin_llm_request_log, create_imported_request, delete_game_by_request_id,
    finish_llm_request_log, get_request_owner,
    get_shared_record_meta_by_request_id, record_visit,
    save_processed_response, set_request_template_source, set_share_status, upsert_shared_record,
    AppState, DbError,
};
use crate::llm;
use crate::ldag;
use crate::types::{BluePrint, BluePrintAct, LDAGNode, Story};

use crate::prompt::{
    clean_json, construct_blueprint_prompt, construct_expand_character_prompt,
    construct_expand_worldview_prompt, construct_fill_node_content_prompt,
    construct_prompt,
};
use crate::sensitive::SensitiveFilter;
use crate::template::{
    normalize_character_ids, normalize_template_endings,
    normalize_template_nodes, sanitize_template_graph,
    convert_story_to_template
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

fn error_response(
    code: impl Into<String>,
    msg: impl Into<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let code_str = code.into();
    let status = match code_str.as_str() {
        CODE_TOO_MANY_REQUESTS | "SERVICE_BUSY" => StatusCode::TOO_MANY_REQUESTS,
        CODE_BAD_REQUEST | CODE_INVALID_BASE_URL => StatusCode::BAD_REQUEST,
        "FORBIDDEN" => StatusCode::FORBIDDEN,
        "NOT_FOUND" => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ApiResponse {
            code: code_str,
            msg: msg.into(),
            data: None,
        }),
    )
}

fn db_error_response(e: DbError) -> (StatusCode, Json<ApiResponse<()>>) {
    error_response(e.code(), e.message())
}

fn rate_limit_response(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<()>>) {
    error_response(CODE_TOO_MANY_REQUESTS, msg)
}

fn error_response_with_data<T: Serialize>(
    code: impl Into<String>,
    msg: impl Into<String>,
    data: T,
) -> (StatusCode, Json<ApiResponse<T>>) {
    let code_str = code.into();
    let status = match code_str.as_str() {
        CODE_TOO_MANY_REQUESTS | "SERVICE_BUSY" => StatusCode::TOO_MANY_REQUESTS,
        CODE_BAD_REQUEST | CODE_INVALID_BASE_URL => StatusCode::BAD_REQUEST,
        "FORBIDDEN" => StatusCode::FORBIDDEN,
        "NOT_FOUND" => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ApiResponse {
            code: code_str,
            msg: msg.into(),
            data: Some(data),
        }),
    )
}

fn sanitize_text(filter: &SensitiveFilter, text: &str) -> String {
    filter.sanitize_str(text).0
}

fn truncate_middle(text: &str, max_chars: usize) -> String {
    let len = text.chars().count();
    if len <= max_chars {
        return text.to_string();
    }
    let head = max_chars / 2;
    let tail = max_chars.saturating_sub(head);
    let prefix: String = text.chars().take(head).collect();
    let suffix: String = text
        .chars()
        .rev()
        .take(tail)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{}…<truncated {} chars>…{}", prefix, len - max_chars, suffix)
}

#[derive(serde::Serialize)]
struct ActFillError {
    act: usize,
    error: String,
    raw_response_preview: String,
    cleaned_response_preview: String,
}

#[allow(clippy::result_large_err)]
fn sanitize_json_value(
    filter: &SensitiveFilter,
    mut value: serde_json::Value,
) -> serde_json::Value {
    filter.sanitize_json(&mut value);
    value
}

fn ensure_not_sensitive<T: Serialize>(
    filter: &SensitiveFilter,
    text: &str,
    field_name: &str,
    original_payload: &T,
) -> Result<(), Response> {
    // Use sanitize_str to check if any replacement actually happens.
    // This ensures consistency: we only error if we would have replaced something.
    let (cleaned, count) = filter.sanitize_str(text);
    if count > 0 && cleaned.contains('*') {
        // Sanitize the payload for error response
        let mut v = serde_json::to_value(original_payload)
            .map_err(|_| error_response(CODE_BAD_REQUEST, "Invalid payload").into_response())?;
        filter.sanitize_json(&mut v);

        // Debug log to see why it matched
        println!("Sensitive filter blocked '{}' -> '{}'", text, cleaned);

        return Err(error_response_with_data(
            CODE_BAD_REQUEST,
            format!("{}包含敏感词，请修改后重试", field_name),
            v,
        )
        .into_response());
    }
    Ok(())
}

fn sanitize_request_payload<T: Serialize + DeserializeOwned>(
    filter: &SensitiveFilter,
    payload: T,
) -> Result<T, Response> {
    let mut v = serde_json::to_value(payload)
        .map_err(|_| error_response(CODE_BAD_REQUEST, "Invalid payload").into_response())?;

    // We only sanitize string values recursively, we should NOT fail if sensitive words are found.
    // sanitize_json modifies the value in place and returns the count of replacements.
    // We ignore the return value because we want to proceed even if replacements occurred.
    filter.sanitize_json(&mut v);

    serde_json::from_value(v)
        .map_err(|_| error_response(CODE_BAD_REQUEST, "Invalid payload").into_response())
}

fn is_trusted_proxy_hop(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.segments()[0] & 0xfe00 == 0xfc00
                || v6.segments()[0] & 0xffc0 == 0xfe80
        }
    }
}

fn normalize_ip_candidate(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if s.parse::<IpAddr>().is_ok() {
        return Some(s.to_string());
    }

    if s.contains('.') {
        if let Some((maybe_ip, maybe_port)) = s.rsplit_once(':') {
            if maybe_port.chars().all(|c| c.is_ascii_digit()) && maybe_ip.parse::<IpAddr>().is_ok()
            {
                return Some(maybe_ip.to_string());
            }
        }
    }

    None
}

fn resolve_client_ip(headers: &HeaderMap, addr: &SocketAddr) -> String {
    let peer_ip = addr.ip();

    if !is_trusted_proxy_hop(peer_ip) {
        return peer_ip.to_string();
    }

    let candidate = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(normalize_ip_candidate)
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').next())
                .and_then(normalize_ip_candidate)
        });

    candidate.unwrap_or_else(|| peer_ip.to_string())
}

fn is_owner_ip(owner_ip: &str, request_ip: &str) -> bool {
    owner_ip == request_ip
        || (owner_ip == "127.0.0.1" && request_ip == "::1")
        || (owner_ip == "::1" && request_ip == "127.0.0.1")
}

fn resolve_llm_api_key(override_key: Option<&str>) -> Result<String, StatusCode> {
    let from_req = override_key.unwrap_or("").trim();
    if !from_req.is_empty() {
        return Ok(from_req.to_string());
    }
    std::env::var("DEEPSEEK_API_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn resolve_llm_model(override_model: Option<&str>) -> String {
    let from_req = override_model.unwrap_or("").trim();
    if !from_req.is_empty() {
        return from_req.to_string();
    }
    crate::llm::DEEPSEEK_DEFAULT_MODEL.to_string()
}

fn resolve_llm_endpoint(base_url: Option<&str>) -> Result<String, StatusCode> {
    let raw = base_url.unwrap_or("").trim();
    if raw.is_empty() {
        return Ok(crate::llm::DEEPSEEK_API_URL.to_string());
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
    State(_state): State<AppState>,
    Json(payload): Json<GenerateRequest>,
) -> Result<Json<ApiResponse<String>>, Response> {
    let prompt = construct_prompt(&payload);
    Ok(success_response(prompt))
}

pub(crate) async fn import_template(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<ImportTemplateRequest>,
) -> Result<Json<ApiResponse<ImportResponse>>, Response> {
    // Check strict fields FIRST
    if let Some(theme) = &payload.theme {
        if theme.chars().count() > 20 {
            return Err(error_response(CODE_BAD_REQUEST, "主题长度不能超过 20 字").into_response());
        }
        ensure_not_sensitive(&state.sensitive, theme, "主题", &payload)?;
    }
    if payload.template.title.chars().count() > 20 {
        return Err(error_response(CODE_BAD_REQUEST, "标题长度不能超过 20 字").into_response());
    }
    ensure_not_sensitive(&state.sensitive, &payload.template.title, "标题", &payload)?;

    // Validate base64 image size
    if let Some(bg) = &payload.template.background_image_base64 {
        if bg.len() > 400_000 { // Approx 300KB
            return Err(error_response(CODE_BAD_REQUEST, "背景图片过大 (超过 300KB)").into_response());
        }
    }
    for char in payload.template.characters.values() {
        if let Some(avatar) = &char.avatar_path {
            if avatar.len() > 400_000 {
                return Err(error_response(CODE_BAD_REQUEST, format!("角色 {} 头像过大 (超过 300KB)", char.name)).into_response());
            }
        }
    }

    // Then sanitize the whole payload (this will replace sensitive words in non-strict fields with *)
    let payload = sanitize_request_payload(&state.sensitive, payload)?;

    let client_ip = resolve_client_ip(&headers, &addr);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let mut request_payload = serde_json::to_value(&payload).unwrap_or(json!({}));
    state.sensitive.sanitize_json(&mut request_payload);

    let mut template = payload.template;

    if let Some(theme) = payload
        .theme
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        template.title = theme.to_string();
        template.meta.logline = theme.to_string();
    }

    if let Some(synopsis) = payload
        .synopsis
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        template.meta.synopsis = synopsis.to_string();
    }

    if let Some(language) = payload
        .language
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        template.meta.language = language.to_string();
    }

    if let Some(genre_list) = payload.genre.as_ref() {
        let cleaned: Vec<String> = genre_list
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if !cleaned.is_empty() {
            template.meta.genre = cleaned.join(" / ");
        }
    }

    if template.characters.is_empty() {
        crate::template::enforce_character_consistency(&mut template, payload.characters.clone());
    }

    normalize_character_ids(&mut template);
    normalize_template_endings(&mut template);
    sanitize_template_graph(&mut template);
    normalize_template_nodes(&mut template);



    let mut processed_response = serde_json::to_value(&template).unwrap_or(json!({}));
    processed_response = sanitize_json_value(&state.sensitive, processed_response);
    if let Ok(t) = serde_json::from_value::<crate::types::MovieTemplate>(processed_response.clone())
    {
        template = t;
    }

    let id = create_imported_request(
        &state.db,
        &client_ip,
        user_agent,
        request_payload,
        processed_response,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    Ok(success_response(ImportResponse { id, template }))
}

pub(crate) async fn share_game(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<ShareRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, Response> {
    let payload = sanitize_request_payload(&state.sensitive, payload)?;

    let request_info = get_request_owner(&state.db, payload.id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    let Some((owner_ip, status)) = request_info else {
        return Err(error_response("NOT_FOUND", "Game not found").into_response());
    };

    if status != "success" {
        return Err(
            error_response("FORBIDDEN", "Game generation not successful, cannot share")
                .into_response(),
        );
    }

    let request_ip = resolve_client_ip(&headers, &addr);
    let is_owner = is_owner_ip(&owner_ip, &request_ip);

    if !is_owner {
        return Err(
            error_response("FORBIDDEN", "You are not the owner of this game").into_response(),
        );
    }

    let shared_record_id = if payload.shared {
        let ua = headers
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .filter(|s| !s.trim().is_empty());

        let _id = upsert_shared_record(&state.db, payload.id, &request_ip, ua)
            .await
            .map_err(|e| db_error_response(e).into_response())?;

        // Return request_id as the identifier for the client to store
        Some(payload.id)
    } else {
        // Just return the request_id
        Some(payload.id)
    };

    set_share_status(&state.db, payload.id, payload.shared)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    Ok(success_response(json!({
        "sharedRecordId": shared_record_id // Use the same key to minimize frontend breakage, but value is request_id now
    })))
}

pub(crate) async fn update_template(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<UpdateTemplateRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, Response> {
    if payload.template.title.chars().count() > 20 {
        return Err(error_response(CODE_BAD_REQUEST, "标题长度不能超过 20 字").into_response());
    }
    ensure_not_sensitive(&state.sensitive, &payload.template.title, "标题", &payload)?;

    // Validate base64 image size
    if let Some(bg) = &payload.template.background_image_base64 {
        if bg.len() > 400_000 {
            return Err(error_response(CODE_BAD_REQUEST, "背景图片过大 (超过 300KB)").into_response());
        }
    }
    for char in payload.template.characters.values() {
        if let Some(avatar) = &char.avatar_path {
            if avatar.len() > 400_000 {
                return Err(error_response(CODE_BAD_REQUEST, format!("角色 {} 头像过大 (超过 300KB)", char.name)).into_response());
            }
        }
    }

    let payload = sanitize_request_payload(&state.sensitive, payload)?;

    let request_info = get_request_owner(&state.db, payload.id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    let Some((owner_ip, status)) = request_info else {
        return Err(error_response("NOT_FOUND", "Game not found").into_response());
    };

    if status != "success" {
        return Err(
            error_response("FORBIDDEN", "Game generation not successful, cannot update")
                .into_response(),
        );
    }

    let request_ip = resolve_client_ip(&headers, &addr);
    let is_owner = is_owner_ip(&owner_ip, &request_ip);

    if !is_owner {
        return Err(
            error_response("FORBIDDEN", "You are not the owner of this game").into_response(),
        );
    }

    let mut template = payload.template;

    normalize_character_ids(&mut template);
    normalize_template_endings(&mut template);
    sanitize_template_graph(&mut template);
    normalize_template_nodes(&mut template);



    let mut template_value = serde_json::to_value(&template).unwrap_or(json!({}));
    template_value = sanitize_json_value(&state.sensitive, template_value);

    save_processed_response(&state.db, payload.id, &template_value)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    if payload
        .source
        .as_deref()
        .is_some_and(|s| s.trim().eq_ignore_ascii_case("import"))
    {
        set_request_template_source(&state.db, payload.id, "import")
            .await
            .map_err(|e| db_error_response(e).into_response())?;
    }

    Ok(success_response(template_value))
}

pub(crate) async fn delete_template(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<DeleteTemplateRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, Response> {
    let payload = sanitize_request_payload(&state.sensitive, payload)?;

    let request_info = get_request_owner(&state.db, payload.id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    let Some((owner_ip, _status)) = request_info else {
        return Err(error_response("NOT_FOUND", "Game not found").into_response());
    };

    let request_ip = resolve_client_ip(&headers, &addr);
    let is_owner = is_owner_ip(&owner_ip, &request_ip);

    if !is_owner {
        return Err(
            error_response("FORBIDDEN", "You are not the owner of this game").into_response(),
        );
    }

    delete_game_by_request_id(&state.db, payload.id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    Ok(success_response(json!({
        "deleted": true
    })))
}

pub(crate) async fn get_shared_game(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, Response> {
    let row = crate::db::get_game_for_play(&state.db, id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    let Some((data, shared, owner_ip)) = row else {
        return Err(error_response("NOT_FOUND", "Game not found").into_response());
    };

    let request_ip = resolve_client_ip(&headers, &addr);
    let is_owner = is_owner_ip(&owner_ip, &request_ip);

    if !shared && !is_owner {
        return Err(error_response("NOT_FOUND", "Game not found").into_response());
    }

    // 2. Record visit (async, fire and forget)
    let db = state.db.clone();
    let client_ip = resolve_client_ip(&headers, &addr);
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    let referer = headers
        .get("referer")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    tokio::spawn(async move {
        if let Err(e) = record_visit(&db, id, &client_ip, &user_agent, referer.as_deref()).await {
            eprintln!("Failed to record visit: {}", e);
        }
    });

    // Remove filtering on game data as per user request
    Ok(success_response(data))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedRecordListItem {
    request_id: Uuid,
    title: String,
    shared_at: String,
    shared: bool,
    synopsis: String,
    genre: String,
    language: String,
    play_count: i64,
}

pub(crate) async fn get_shared_record_meta(
    State(state): State<AppState>,
    Path(request_id): Path<Uuid>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, Response> {
    let request_ip = resolve_client_ip(&headers, &addr);

    let meta = get_shared_record_meta_by_request_id(&state.db, request_id)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    let Some((shared, shared_at, owner_ip)) = meta else {
        return Err(error_response("NOT_FOUND", "Record not found").into_response());
    };

    let is_owner = is_owner_ip(&owner_ip, &request_ip);

    Ok(success_response(json!({
        // "sharedRecordId": ... REMOVED per security requirement
        "requestId": request_id,
        "shared": shared,
        "sharedAt": shared_at.map(|v| json!(v)).unwrap_or(serde_json::Value::Null),
        "isOwner": is_owner
    })))
}

pub(crate) async fn list_records(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<RecordsListRequest>,
) -> Result<Json<ApiResponse<Vec<SharedRecordListItem>>>, Response> {
    let payload = sanitize_request_payload(&state.sensitive, payload)?;

    let owner_ip = resolve_client_ip(&headers, &addr);

    if payload.ids.is_empty() {
        return Ok(success_response(Vec::<SharedRecordListItem>::new()));
    }

    if payload.ids.len() > 200 {
        return Err(error_response(CODE_BAD_REQUEST, "Too many ids").into_response());
    }

    // payload.ids are now treated as request_ids
    let rows = crate::db::list_shared_records_by_request_ids(&state.db, &payload.ids, &owner_ip)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            db_error_response(DbError::InternalError).into_response()
        })?;

    let mut items = rows
        .into_iter()
        .map(
            |(request_id, shared_at, shared, title, synopsis, genre, language, play_count)| {
                SharedRecordListItem {
                    request_id,
                    title: title.unwrap_or_else(|| "Untitled".to_string()),
                    shared_at,
                    shared,
                    synopsis: synopsis.unwrap_or_default(),
                    genre: genre.unwrap_or_default(),
                    language: language.unwrap_or_default(),
                    play_count,
                }
            },
        )
        .collect::<Vec<_>>();

    for item in items.iter_mut() {
        item.title = sanitize_text(&state.sensitive, &item.title);
        item.synopsis = sanitize_text(&state.sensitive, &item.synopsis);
        item.genre = sanitize_text(&state.sensitive, &item.genre);
        item.language = sanitize_text(&state.sensitive, &item.language);
    }

    Ok(success_response(items))
}

pub(crate) async fn generate(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<GenerateRequest>,
) -> Result<Response, Response> {
    if let Some(theme) = &payload.theme {
        ensure_not_sensitive(&state.sensitive, theme, "主题", &payload)?;
    }
    if let Some(free_input) = &payload.free_input {
         ensure_not_sensitive(&state.sensitive, free_input, "自由输入", &payload)?;
    }

    let payload = sanitize_request_payload(&state.sensitive, payload)?;
    let client_ip = resolve_client_ip(&headers, &addr);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let using_override_key = payload.api_key.as_ref().is_some_and(|k| !k.trim().is_empty());
    
    // Step 1: Static Data
    let (level_count, act_count) = ldag::generate_static_data();
    
    // Step 2: BluePrint Generation
    let blueprint_prompt = construct_blueprint_prompt(&payload, level_count, act_count);
    
    // Log start
    let mut payload_json = serde_json::to_value(&payload).unwrap_or(json!({}));
    if let Some(obj) = payload_json.as_object_mut() {
        obj.remove("apiKey");
    }
    state.sensitive.sanitize_json(&mut payload_json);
    let prompt_for_log = sanitize_text(&state.sensitive, &blueprint_prompt);
    
    let request_id = begin_llm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/generate",
        payload_json,
        &prompt_for_log,
        using_override_key,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let db = state.db.clone();
    let _sensitive = state.sensitive.clone();
    let payload_clone = payload.clone();

    let handle = tokio::spawn(async move {
        let start = std::time::Instant::now();
        
        // Helper to call LLM
        let call_llm = |prompt: String, json_mode: bool| {
            let payload_for_future = payload_clone.clone();
            async move {
                let base_url = payload_for_future
                    .base_url
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("");

                let model = resolve_llm_model(payload_for_future.model.as_deref());

                let api_key = payload_for_future
                    .api_key
                    .clone()
                    .filter(|s| !s.is_empty());

                crate::llm::call_llm_with_api_key(
                    prompt,
                    json_mode,
                    api_key,
                    Some(base_url.to_string()),
                    Some(model.to_string()),
                )
                .await
            }
        };

        // 2.1 Call LLM for BluePrint
        println!("Generating BluePrint...");
        let blueprint_resp = match call_llm(blueprint_prompt, true).await {
            Ok(s) => s,
            Err(e) => {
                finish_llm_request_log(&db, request_id, "failed", None, Some(&e), None).await;
                return Err(error_response(CODE_INTERNAL_ERROR, format!("BluePrint Gen Failed: {}", e)).into_response());
            }
        };
        
        let blueprint_clean = clean_json(&blueprint_resp);
        let mut blueprint: BluePrint = match serde_json::from_str(&blueprint_clean) {
            Ok(v) => v,
            Err(json_err) => match json5::from_str(&blueprint_clean) {
                Ok(v) => v,
                Err(json5_err) => {
                    let err_msg = format!(
                        "BluePrint Parse Error: serde_json={}, json5={}",
                        json_err, json5_err
                    );
                    finish_llm_request_log(&db, request_id, "failed", Some(&blueprint_resp), Some(&err_msg), None).await;
                    return Err(error_response(CODE_INTERNAL_ERROR, err_msg).into_response());
                }
            },
        };
        
        println!("BluePrint generated. Levels: {}, Acts: {}", blueprint.level_count, blueprint.act_count);

        // Step 3: LDAG Generation
        let ldag_acts_structure = match ldag::generate_ldag(&blueprint) {
            Ok(v) => v,
            Err(e) => {
                finish_llm_request_log(&db, request_id, "failed", None, Some(&e), None).await;
                return Err(error_response(CODE_INTERNAL_ERROR, format!("LDAG Gen Failed: {}", e)).into_response());
            }
        };
        
        // Step 4: Fill Content (Concurrent)
        let msg = format!("Filling content for {} acts...", ldag_acts_structure.len());
        println!("{}", msg);
        crate::llm::log_to_file(&msg);
        let mut filled_acts_futures = Vec::new();
        
        for (idx, act_nodes_structure) in ldag_acts_structure.iter().enumerate() {
            let act_info = blueprint.acts.get(idx).cloned().unwrap_or_else(|| BluePrintAct {
                level_range: (0, 0),
                name: format!("Act {}", idx + 1),
                description: "".to_string(),
            });
            
            let nodes_structure = act_nodes_structure.clone();
            let payload_ref = payload_clone.clone();
            
            // Spawn task for this act
            // We need to clone the closure or define logic here?
            // Closure `call_llm` captures variables, can't be easily cloned if it captures `payload_clone`.
            // We'll just replicate logic or use Arc.
            // Simplified: just call the function directly.
            
            let base_url = Some(payload_clone.base_url
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or("")
                .to_string());

            let model = Some(resolve_llm_model(payload_clone.model.as_deref()));

            let api_key = payload_clone.api_key
                .clone()
                .filter(|s| !s.is_empty());

            let sensitive = _sensitive.clone();
            
            filled_acts_futures.push(tokio::spawn(async move {
                let prompt = construct_fill_node_content_prompt(&payload_ref, &act_info, &nodes_structure);
                let act = idx + 1;
                
                let resp = crate::llm::call_llm_with_api_key(
                    prompt,
                    true,
                    api_key,
                    base_url,
                    model,
                )
                .await
                .map_err(|e| ActFillError {
                    act,
                    error: format!("LLM Call Failed: {}", e),
                    raw_response_preview: String::new(),
                    cleaned_response_preview: String::new(),
                })?;
                
                #[derive(serde::Deserialize)]
                struct WrappedLDAGNodes {
                    nodes: Vec<Vec<LDAGNode>>,
                }

                let raw_s = sanitize_text(sensitive.as_ref(), &resp);
                let act_clean = clean_json(&resp);
                let clean_s = sanitize_text(sensitive.as_ref(), &act_clean);

                let parsed_nodes = match serde_json::from_str::<WrappedLDAGNodes>(&act_clean) {
                    Ok(v) => Ok(v.nodes),
                    Err(wrapped_json_err) => match json5::from_str::<WrappedLDAGNodes>(&act_clean) {
                        Ok(v) => Ok(v.nodes),
                        Err(wrapped_json5_err) => {
                            let value_json = serde_json::from_str::<serde_json::Value>(&act_clean)
                                .map_err(|e| e.to_string());
                            let value_json5 = json5::from_str::<serde_json::Value>(&act_clean)
                                .map_err(|e| e.to_string());

                            let maybe_value = value_json.or(value_json5);
                            match maybe_value {
                                Ok(v) => {
                                    if v.is_array() {
                                        serde_json::from_value::<Vec<Vec<LDAGNode>>>(v)
                                            .map_err(|e| e.to_string())
                                    } else if let Some(nodes_v) = v.get("nodes") {
                                        serde_json::from_value::<Vec<Vec<LDAGNode>>>(nodes_v.clone())
                                            .map_err(|e| e.to_string())
                                    } else {
                                        Err("Missing field 'nodes'".to_string())
                                    }
                                }
                                Err(value_err) => Err(format!(
                                    "JSON Parse Error for Act {}: serde_json={}, json5={}, value_parse={}",
                                    act, wrapped_json_err, wrapped_json5_err, value_err
                                )),
                            }
                        }
                    },
                };

                match parsed_nodes {
                    Ok(nodes) => Ok::<Vec<Vec<LDAGNode>>, ActFillError>(nodes),
                    Err(err) => Err(ActFillError {
                        act,
                        error: err,
                        raw_response_preview: truncate_middle(&raw_s, 6000),
                        cleaned_response_preview: truncate_middle(&clean_s, 6000),
                    }),
                }
            }));
        }
        
        // Wait for all acts
        let mut filled_act_list = Vec::new();
        for (i, f) in filled_acts_futures.into_iter().enumerate() {
            let msg = format!("Waiting for Act {} content...", i + 1);
            println!("{}", msg);
            crate::llm::log_to_file(&msg);
            match f.await {
                Ok(Ok(nodes)) => {
                    let msg = format!("Act {} filled successfully. Nodes count: {}", i + 1, nodes.len());
                    println!("{}", msg);
                    crate::llm::log_to_file(&msg);
                    filled_act_list.push(nodes)
                },
                Ok(Err(e)) => {
                    let msg = format!("Act {} failed with logic error: {}", e.act, e.error);
                    println!("{}", msg);
                    crate::llm::log_to_file(&msg);
                    crate::llm::log_to_file(&format!(
                        "REQUEST {} Act {} raw_response_preview:\n{}",
                        request_id, e.act, e.raw_response_preview
                    ));
                    crate::llm::log_to_file(&format!(
                        "REQUEST {} Act {} cleaned_response_preview:\n{}",
                        request_id, e.act, e.cleaned_response_preview
                    ));

                    let response_log = serde_json::to_string(&e).unwrap_or_default();
                    finish_llm_request_log(&db, request_id, "failed", Some(&response_log), Some(&e.error), None).await;

                    return Err(error_response_with_data(
                        CODE_INTERNAL_ERROR,
                        format!("Content Fill Failed: Act {}", e.act),
                        e,
                    )
                    .into_response());
                }
                Err(e) => {
                    let msg = format!("Act {} failed with task join error: {}", i + 1, e);
                    println!("{}", msg);
                    crate::llm::log_to_file(&msg);
                    finish_llm_request_log(&db, request_id, "failed", None, Some(&format!("Task Join Error: {}", e)), None).await;
                    return Err(error_response(CODE_INTERNAL_ERROR, "Task Join Error").into_response());
                }
            }
        }
        
        // Validate count
        if filled_act_list.len() != ldag_acts_structure.len() {
             eprintln!("Act count mismatch: expected {}, got {}", ldag_acts_structure.len(), filled_act_list.len());
             return Err(error_response(CODE_INTERNAL_ERROR, "Act count mismatch").into_response());
        }
        
        // Step 4.5: Ensure L1N1 matches StartNode
        // The Prompt for Act 1 should have handled this?
        // "Attention: L1N1 must directly use ... startNode content".
        // We trust LLM to follow instruction.
        // Or we force overwrite it here.
        if let Some(first_act) = filled_act_list.first_mut() {
            if let Some(first_layer) = first_act.first_mut() {
                if let Some(l1n1) = first_layer.first_mut() {
                    l1n1.content = blueprint.start_node.content.clone();
                    l1n1.characters = blueprint.start_node.characters.clone();
                    // Choices: we keep structure generated by LDAG, but maybe update content?
                    // BluePrint.startNode.choices has content.
                    // LDAG structure might have different number of choices.
                    // If LDAG generated 2 choices, we can map.
                    // If mismatch, we rely on LLM's filled content for choices.
                    // Let's assume LLM followed instructions.
                }
            }
        }

        // Step 5: Assemble Story
        // Fix ending keys: ensure prefix ENDING_
        let mut fixed_endings = std::collections::HashMap::new();
        for (k, v) in blueprint.endings.iter() {
            let key = if k.starts_with("ENDING_") { k.clone() } else { format!("ENDING_{}", k) };
            fixed_endings.insert(key, v.clone());
        }
        blueprint.endings = fixed_endings;
        
        let meta = crate::types::MetaInfo {
            logline: payload_clone.theme.clone().unwrap_or_default(),
            synopsis: payload_clone.synopsis.clone().unwrap_or_default(),
            genre: payload_clone.genre.clone().unwrap_or_default().join(" / "),
            language: payload_clone.language.clone().unwrap_or("zh-CN".to_string()),
            target_runtime_minutes: 0, // Calculated later?
        };

        // Convert payload characters to Character map
        let mut characters_map = std::collections::HashMap::new();
        if let Some(chars) = &payload_clone.characters {
            for c in chars {
                let id = c.name.clone(); // Use name as ID for now
                let avatar_path = None;
                characters_map.insert(id.clone(), crate::types::Character {
                    id,
                    name: c.name.clone(),
                    gender: c.gender.clone(),
                    age: 0,
                    role: c.description.clone(),
                    background: "".to_string(),
                    avatar_path,
                });
            }
        }

        let story = Story {
            request_id: Some(request_id.to_string()),
            title: payload_clone.theme.clone().unwrap_or_else(|| "Unknown".to_string()),
            meta,
            characters: characters_map,
            blueprint,
            act_list: filled_act_list,
        };
        
        // Step 6: Convert to MovieTemplate for Frontend
        // println!("Converting story to template...");
        // let mut template = convert_story_to_template(story.clone(), uuid::Uuid::new_v4().to_string(), "User".to_string());

        // Step 7: Enforce Character Consistency
        // Ensure the returned characters match exactly what the frontend requested
        // println!("Enforcing character consistency...");
        // crate::template::enforce_character_consistency(&mut template, payload_clone.characters.clone());

        // Save
        println!("Saving processed response to DB...");
        let story_value = serde_json::to_value(&story).unwrap_or(json!({}));
        if let Err(e) = save_processed_response(&db, request_id, &story_value).await {
            eprintln!("Failed to save story: {}", e);
        }
        println!("Story saved successfully.");
        
        let duration = start.elapsed();
        finish_llm_request_log(
            &db,
            request_id,
            "success",
            None, // Don't log full story content to raw log to save space/privacy
            None,
            Some(duration.as_millis().min(i64::MAX as u128) as i64),
        )
        .await;
        
        Ok(success_response(GenerateResponse {
            id: request_id,
            story,
        }).into_response())
    });

    match handle.await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Task join error: {}", e);
            Err(error_response(CODE_INTERNAL_ERROR, "Internal Server Error").into_response())
        }
    }
}

pub(crate) async fn expand_worldview_prompt(
    State(_state): State<AppState>,
    Json(req): Json<ExpandWorldviewRequest>,
) -> Result<Json<ApiResponse<String>>, Response> {
    let prompt = construct_expand_worldview_prompt(&req);
    Ok(success_response(prompt))
}

pub(crate) async fn expand_character_prompt(
    State(_state): State<AppState>,
    Json(req): Json<ExpandCharacterRequest>,
) -> Result<Json<ApiResponse<String>>, Response> {
    let prompt = construct_expand_character_prompt(&req);
    Ok(success_response(prompt))
}

pub(crate) async fn expand_worldview(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExpandWorldviewRequest>,
) -> Result<Response, Response> {
    ensure_not_sensitive(&state.sensitive, &req.theme, "主题", &req)?;
    let req = sanitize_request_payload(&state.sensitive, req)?;

    let client_ip = resolve_client_ip(&headers, &addr);

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let _language = req.language.as_deref().unwrap_or("zh-CN");
    let prompt = construct_expand_worldview_prompt(&req);

    let using_override_key = req.api_key.as_ref().is_some_and(|k| !k.trim().is_empty());
    let mut payload_json = serde_json::to_value(&req).unwrap_or(json!({}));
    if let Some(obj) = payload_json.as_object_mut() {
        obj.remove("apiKey");
    }

    state.sensitive.sanitize_json(&mut payload_json);
    let prompt_for_log = sanitize_text(&state.sensitive, &prompt);

    // Initialize Client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(240))
        .build()
        .map_err(|e| error_response(CODE_INTERNAL_ERROR, e.to_string()).into_response())?;

    let request_id = begin_llm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/expand/worldview",
        payload_json,
        &prompt_for_log,
        using_override_key,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let db = state.db.clone();
    let sensitive = state.sensitive.clone();
    let req_clone = req.clone();

    let handle = tokio::spawn(async move {
        let start = std::time::Instant::now();
        let base_url = req_clone
            .base_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("");

        let endpoint = match resolve_llm_endpoint(Some(base_url)) {
            Ok(v) => v,
            Err(_) => {
                let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                finish_llm_request_log(
                    &db,
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

        let api_key_candidate = req_clone
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let api_key = match api_key_candidate {
            Some(v) => v,
            None => match resolve_llm_api_key(None) {
                Ok(v) => v,
                Err(_) => {
                    let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                    finish_llm_request_log(
                        &db,
                        request_id,
                        "failed",
                        None,
                        Some("Missing API Key"),
                        Some(response_time_ms),
                    )
                    .await;
                    return Err(
                        error_response("API_KEY_REQUIRED", "API Key is required").into_response()
                    );
                }
            },
        };

        let model = resolve_llm_model(req_clone.model.as_deref());

        let messages = vec![
            json!({
                "role": "system",
                "content": "You are a professional interactive movie scriptwriter and game designer."
            }),
            json!({
                "role": "user",
                "content": prompt
            }),
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

        let response = match client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("LLM Request failed: {}", e);
                let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some("LLM Request failed"),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(CODE_INTERNAL_ERROR, "LLM Request failed").into_response());
            }
        };

        let duration = start.elapsed();
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let error_text_s = sanitize_text(&sensitive, &error_text);
            eprintln!("LLM Error: {}", error_text_s);

            if llm::is_rate_limit_error(&error_text) {
                let error_message = if let Some(code) = llm::extract_llm_error_code(&error_text) {
                    format!("LLM API 返回错误码 {}: {}", code, error_text_s)
                } else {
                    error_text_s.clone()
                };

                finish_llm_request_log(
                    &db,
                    request_id,
                    "error",
                    None,
                    Some(&error_text_s),
                    Some(response_time_ms),
                )
                .await;
                return Err(rate_limit_response(error_message).into_response());
            }

            if llm::contains_limit(&error_text) {
                finish_llm_request_log(
                    &db,
                    request_id,
                    "error",
                    None,
                    Some(&error_text_s),
                    Some(response_time_ms),
                )
                .await;
                return Err(rate_limit_response(&error_text_s).into_response());
            }

            finish_llm_request_log(
                &db,
                request_id,
                "error",
                None,
                Some(&error_text_s),
                Some(response_time_ms),
            )
            .await;

            return Err(error_response(CODE_INTERNAL_ERROR, error_text_s).into_response());
        }

        let text_response = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some(&format!("Failed to read response body: {}", e)),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(
                    CODE_INTERNAL_ERROR,
                    format!("Failed to read response body: {}", e),
                )
                .into_response());
            }
        };

        // Try to parse as generic JSON first to check for "error" field
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text_response) {
            if json_value.get("error").is_some() {
                let text_response_s = sanitize_text(&sensitive, &text_response);
                println!(
                    "LLM returned 200 OK but with error body: {}",
                    text_response_s
                );
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    Some(&text_response_s),
                    Some("LLM Logic Error"),
                    Some(response_time_ms),
                )
                .await;
                return Err(
                    error_response(CODE_INTERNAL_ERROR, "LLM Logic Error").into_response()
                );
            }
        }

        // Extract content from chat response
        let response_json: serde_json::Value = match serde_json::from_str(&text_response) {
            Ok(v) => v,
            Err(e) => {
                let text_response_s = sanitize_text(&sensitive, &text_response);
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    Some(&text_response_s),
                    Some(&format!("Failed to parse LLM response JSON: {}", e)),
                    Some(response_time_ms),
                )
                .await;
                return Err(
                    error_response(CODE_INTERNAL_ERROR, "Failed to parse LLM response").into_response(),
                );
            }
        };

        let content = match response_json["choices"][0]["message"]["content"].as_str() {
            Some(c) => c.to_string(),
            None => {
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some("Invalid LLM response structure"),
                    Some(response_time_ms),
                )
                .await;
                return Err(
                    error_response(CODE_INTERNAL_ERROR, "Invalid LLM response structure")
                        .into_response(),
                );
            }
        };

        // Log raw content as per user demand
        finish_llm_request_log(
            &db,
            request_id,
            "success",
            Some(&content),
            None,
            Some(response_time_ms),
        )
        .await;

        // Return original content to frontend, log raw content to DB
        Ok(success_response(content).into_response())
    });

    match handle.await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Task join error: {}", e);
            Err(error_response(CODE_INTERNAL_ERROR, "Internal Server Error").into_response())
        }
    }
}

pub(crate) async fn expand_character(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExpandCharacterRequest>,
) -> Result<Response, Response> {
    ensure_not_sensitive(&state.sensitive, &req.theme, "主题", &req)?;
    let req = sanitize_request_payload(&state.sensitive, req)?;

    let client_ip = resolve_client_ip(&headers, &addr);

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

    state.sensitive.sanitize_json(&mut payload_json);
    let prompt_for_log = sanitize_text(&state.sensitive, &prompt);

    let request_id = begin_llm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/expand/character",
        payload_json,
        &prompt_for_log,
        using_override_key,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let db = state.db.clone();
    let sensitive = state.sensitive.clone();
    let req_clone = req.clone();

    let handle = tokio::spawn(async move {
        let start = std::time::Instant::now();
        let base_url = req_clone
            .base_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("");

        let endpoint = match resolve_llm_endpoint(Some(base_url)) {
            Ok(v) => v,
            Err(_) => {
                let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                finish_llm_request_log(
                    &db,
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

        let api_key_candidate = req_clone
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let api_key = match api_key_candidate {
            Some(v) => v,
            None => match resolve_llm_api_key(None) {
                Ok(v) => v,
                Err(_) => {
                    let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                    finish_llm_request_log(
                        &db,
                        request_id,
                        "failed",
                        None,
                        Some("Missing API Key"),
                        Some(response_time_ms),
                    )
                    .await;
                    return Err(
                        error_response("API_KEY_REQUIRED", "API Key is required").into_response()
                    );
                }
            },
        };

        let model = resolve_llm_model(req_clone.model.as_deref());

        let messages = vec![
            json!({
                "role": "system",
                "content": "You are a professional interactive movie scriptwriter and game designer. Output strictly valid JSON."
            }),
            json!({
                "role": "user",
                "content": prompt
            }),
        ];

        let request_body = json!({
            "model": model,
            "messages": messages,
            "response_format": { "type": "json_object" }, // Force JSON for character expansion
            "temperature": 1,
            "top_p": 0.95,
            "max_tokens": 8192
        });

        let response = match client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("LLM Request failed: {}", e);
                let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some("LLM Request failed"),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(CODE_INTERNAL_ERROR, "LLM Request failed").into_response());
            }
        };

        let duration = start.elapsed();
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let error_text_s = sanitize_text(&sensitive, &error_text);
            eprintln!("LLM Error: {}", error_text_s);

            if llm::is_rate_limit_error(&error_text) {
                let error_message = if let Some(code) = llm::extract_llm_error_code(&error_text) {
                    format!("LLM API 返回错误码 {}: {}", code, error_text_s)
                } else {
                    error_text_s.clone()
                };

                finish_llm_request_log(
                    &db,
                    request_id,
                    "error",
                    None,
                    Some(&error_text_s),
                    Some(response_time_ms),
                )
                .await;
                return Err(rate_limit_response(error_message).into_response());
            }

            if llm::contains_limit(&error_text) {
                finish_llm_request_log(
                    &db,
                    request_id,
                    "error",
                    None,
                    Some(&error_text_s),
                    Some(response_time_ms),
                )
                .await;
                return Err(rate_limit_response(&error_text_s).into_response());
            }

            finish_llm_request_log(
                &db,
                request_id,
                "error",
                None,
                Some(&error_text_s),
                Some(response_time_ms),
            )
            .await;
            return Err(error_response(CODE_INTERNAL_ERROR, error_text_s).into_response());
        }

        let text_response = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some(&format!("Failed to read response body: {}", e)),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(
                    CODE_INTERNAL_ERROR,
                    format!("Failed to read response body: {}", e),
                )
                .into_response());
            }
        };

        if text_response.trim().is_empty() {
            eprintln!("LLM returned empty response body");
            let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;
            finish_llm_request_log(
                &db,
                request_id,
                "failed",
                Some(""),
                Some("LLM returned empty response body"),
                Some(response_time_ms),
            )
            .await;
            return Err(
                error_response(CODE_INTERNAL_ERROR, "LLM returned empty response body").into_response(),
            );
        }

        // Check for 200 OK error
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text_response) {
            if json_value.get("error").is_some() {
                let text_response_s = sanitize_text(&sensitive, &text_response);
                println!(
                    "LLM returned 200 OK but with error body: {}",
                    text_response_s
                );

                if llm::is_rate_limit_error(&text_response) {
                    let error_message = if let Some(code) = llm::extract_llm_error_code(&text_response)
                    {
                        format!("LLM API 返回错误码 {}: {}", code, text_response_s)
                    } else {
                        text_response_s.clone()
                    };

                    finish_llm_request_log(
                        &db,
                        request_id,
                        "error",
                        None,
                        Some(&text_response_s),
                        Some(response_time_ms),
                    )
                    .await;
                    return Err(rate_limit_response(error_message).into_response());
                }

                finish_llm_request_log(
                    &db,
                    request_id,
                    "error",
                    None,
                    Some(&text_response_s),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(CODE_INTERNAL_ERROR, text_response_s).into_response());
            }
        }

        // Extract content from chat response
        let response_json: serde_json::Value = match serde_json::from_str(&text_response) {
            Ok(v) => v,
            Err(e) => {
                let text_response_s = sanitize_text(&sensitive, &text_response);
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    Some(&text_response_s),
                    Some(&format!("Failed to parse LLM response JSON: {}", e)),
                    Some(response_time_ms),
                )
                .await;
                return Err(
                    error_response(CODE_INTERNAL_ERROR, "Failed to parse LLM response").into_response(),
                );
            }
        };

        let content = match response_json["choices"][0]["message"]["content"].as_str() {
            Some(c) => c,
            None => {
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some("Invalid LLM response structure"),
                    Some(response_time_ms),
                )
                .await;
                return Err(
                    error_response(CODE_INTERNAL_ERROR, "Invalid LLM response structure")
                        .into_response(),
                );
            }
        };

        let clean = clean_json(content);
        match serde_json::from_str::<Vec<CharacterInput>>(&clean) {
            Ok(chars) => {
                let chars_value = serde_json::to_value(&chars).unwrap_or(json!([]));
                // Log raw content as per user demand
                let chars_log = chars_value.to_string();

                finish_llm_request_log(
                    &db,
                    request_id,
                    "success",
                    Some(&chars_log),
                    None,
                    Some(response_time_ms),
                )
                .await;
                // Return original unsanitized chars to frontend
                Ok(success_response(chars).into_response())
            }
            Err(e) => {
                let clean_s = sanitize_text(&sensitive, &clean);
                finish_llm_request_log(
                    &db,
                    request_id,
                    "failed",
                    Some(&clean_s),
                    Some(&format!("Parse Error: {}", e)),
                    Some(response_time_ms),
                )
                .await;
                Err(error_response(CODE_INTERNAL_ERROR, format!("Parse Error: {}", e)).into_response())
            }
        }
    });

    match handle.await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Task join error: {}", e);
            Err(error_response(CODE_INTERNAL_ERROR, "Internal Server Error").into_response())
        }
    }
}
