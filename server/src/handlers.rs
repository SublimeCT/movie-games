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
use uuid::Uuid;

use crate::api_types::{
    CharacterInput, DeleteTemplateRequest, ExpandCharacterRequest, ExpandWorldviewRequest,
    GenerateRequest, GenerateResponse, ImportTemplateRequest, RecordsListRequest, ShareRequest,
    UpdateTemplateRequest,
};
use crate::db::{
    begin_glm_request_log, create_imported_request, delete_game_by_request_id,
    finish_glm_request_log, get_request_owner,
    get_shared_record_meta_by_request_id, record_visit,
    save_processed_response, set_request_template_source, set_share_status, upsert_shared_record,
    AppState, DbError,
};
use crate::llm_client;
use crate::images::{
    ensure_avatar_fallbacks, fallback_background_data_uri, generate_scene_background_base64,
    maybe_attach_generated_avatars, normalize_cogview_size, pick_background_prompt,
};
use crate::prompt::{
    clean_json, construct_expand_character_prompt, construct_expand_worldview_prompt, construct_prompt,
};
use crate::sensitive::SensitiveFilter;
use crate::template::{
    convert_lite_to_full, normalize_character_ids, normalize_template_endings,
    normalize_template_nodes, sanitize_affinity_effects, sanitize_template_graph,
    MovieTemplateLite,
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
) -> Result<Json<ApiResponse<GenerateResponse>>, Response> {
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
    sanitize_affinity_effects(&mut template);

    ensure_avatar_fallbacks(&mut template, payload.characters.as_ref());

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

    Ok(success_response(GenerateResponse { id, template }))
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
    sanitize_affinity_effects(&mut template);

    ensure_avatar_fallbacks(&mut template, None);

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
    // Check free_input as well if it acts as theme
    if let Some(free_input) = &payload.free_input {
         ensure_not_sensitive(&state.sensitive, free_input, "自由输入", &payload)?;
    }

    let payload = sanitize_request_payload(&state.sensitive, payload)?;

    // Validation for numerical constraints
    if let Some(min_nodes) = payload.min_nodes {
        if min_nodes < 45 {
            return Err(error_response(CODE_BAD_REQUEST, "min_nodes must be >= 45").into_response());
        }
    }
    if let Some(max_nodes) = payload.max_nodes {
        if max_nodes > 85 {
            return Err(error_response(CODE_BAD_REQUEST, "max_nodes must be <= 85").into_response());
        }
    }
    if let Some(min_endings) = payload.min_endings {
        if min_endings < 4 {
            return Err(error_response(CODE_BAD_REQUEST, "min_endings must be >= 4").into_response());
        }
    }
    if let Some(max_endings) = payload.max_endings {
        if max_endings > 6 {
            return Err(error_response(CODE_BAD_REQUEST, "max_endings must be <= 6").into_response());
        }
    }

    let client_ip = resolve_client_ip(&headers, &addr);

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let theme = payload
        .theme
        .as_deref()
        .or(payload.free_input.as_deref())
        .unwrap_or("Unknown Theme");
    println!(
        "Received generate request: {:?}",
        sanitize_text(&state.sensitive, theme)
    );

    let prompt = construct_prompt(&payload);
    println!("Prompt constructed.");

    // LLM client setup deferred to spawn block
    
    // We can't log the exact prompt sent to LLM yet because llm_client constructs it (system + user).
    // But we can log the user prompt part which we have.
    let prompt_for_log_part = prompt.clone();
    let start = std::time::Instant::now();

    let using_override_key = false;

    let mut payload_json = serde_json::to_value(&payload).unwrap_or(json!({}));
    state.sensitive.sanitize_json(&mut payload_json);

    let prompt_for_log = sanitize_text(&state.sensitive, &prompt_for_log_part);

    // Resolve model and think status for logging (and usage)
    let effective_think = false; // Disable think mode
    let config = llm_client::resolve_config()
    .map_err(|e| error_response(CODE_BAD_REQUEST, e).into_response())?;
    
    let request_id = begin_glm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/generate",
        payload_json,
        &prompt_for_log,
        using_override_key,
        Some(&config.model),
        effective_think,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let db = state.db.clone();
    let sensitive = state.sensitive.clone();
    let payload_clone = payload.clone();
    let prompt_clone = prompt_for_log_part.clone();

    // Spawn a background task to handle the GLM request and DB updates
    // This ensures the request completes and is recorded even if the client disconnects
    let handle = tokio::spawn(async move {
        // Use the abstracted LLM client
        // This handles endpoint resolution, API key resolution, request construction, and response parsing
        let content = match llm_client::call_llm(
            prompt_clone,
            true, // json_mode
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                finish_glm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some(&e),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(CODE_INTERNAL_ERROR, e).into_response());
            }
        };

        let duration = start.elapsed();
        println!("LLM Request took: {:?}", duration);
        println!("LLM Response Content Length: {}", content.len());

        let clean_json_str = clean_json(&content);
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

        let template_lite: MovieTemplateLite = match serde_json::from_str(&clean_json_str) {
            Ok(t) => {
                println!("JSON deserialization successful. Converting to full template.");
                t
            }
            Err(e) => {
                eprintln!("JSON Error: {}", e);
                let content_s = sanitize_text(&sensitive, &content);
                finish_glm_request_log(
                    &db,
                    request_id,
                    "failed",
                    Some(&content_s),
                    Some(&format!("JSON Parse Error: {}", e)),
                    Some(response_time_ms),
                )
                .await;
                return Err(
                    error_response(CODE_INTERNAL_ERROR, format!("JSON Parse Error: {}", e))
                        .into_response(),
                );
            }
        };

        let language_tag = payload_clone.language.as_deref().unwrap_or("zh-CN");
        let mut template = convert_lite_to_full(template_lite, language_tag);
        normalize_character_ids(&mut template);
        normalize_template_nodes(&mut template);
        normalize_template_endings(&mut template);

        // User insisted: "Must return character info passed by frontend exactly as is"
        crate::template::enforce_character_consistency(&mut template, payload_clone.characters.clone());

        normalize_character_ids(&mut template);
        normalize_template_endings(&mut template);
        sanitize_template_graph(&mut template);
        sanitize_affinity_effects(&mut template);

        // Image generation logic
        let using_override_key = false;

        let should_generate_images = true;

        if should_generate_images {
            // We need API Key for image generation
            if let Ok(api_key) = llm_client::resolve_api_key(None) {
                let client = reqwest::Client::new();
                let size = normalize_cogview_size(payload_clone.size.as_deref());
                let synopsis_for_image = pick_background_prompt(&payload_clone, &template);
                
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
                    payload_clone.characters.as_ref(),
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
        } else {
            template.background_image_base64 = Some(fallback_background_data_uri(
                &template.title,
                &template.meta.synopsis,
            ));
        }

        ensure_avatar_fallbacks(&mut template, payload_clone.characters.as_ref());

        let template_value = serde_json::to_value(&template).unwrap_or(json!({}));

        // Save the processed template (original, not sanitized)
        if let Err(e) = save_processed_response(&db, request_id, &template_value).await {
            eprintln!("Failed to save processed response: {}", e);
        }

        // Log raw content as per user demand
        finish_glm_request_log(
            &db,
            request_id,
            "success",
            Some(&content),
            None,
            Some(response_time_ms),
        )
        .await;

        Ok(success_response(GenerateResponse {
            id: request_id,
            template,
        })
        .into_response())
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

    let using_override_key = false;
    let mut payload_json = serde_json::to_value(&req).unwrap_or(json!({}));
    
    state.sensitive.sanitize_json(&mut payload_json);
    let prompt_for_log = sanitize_text(&state.sensitive, &prompt);

    // Resolve model and think status for logging (and usage)
    let effective_think = false; // Disable think mode
    let config = llm_client::resolve_config()
    .map_err(|e| error_response(CODE_BAD_REQUEST, e).into_response())?;

    let request_id = begin_glm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/expand/worldview",
        payload_json,
        &prompt_for_log,
        using_override_key,
        Some(&config.model),
        effective_think,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let db = state.db.clone();
    let _sensitive = state.sensitive.clone(); // Kept for consistency if needed, but llm_client handles errors
    let handle = tokio::spawn(async move {
        let start = std::time::Instant::now();
        
        let content = match llm_client::call_llm(
            prompt,
            false, // json_mode
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                finish_glm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some(&e),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(CODE_INTERNAL_ERROR, e).into_response());
            }
        };

        let duration = start.elapsed();
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

        // Log raw content as per user demand
        finish_glm_request_log(
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

    let using_override_key = false;
    let mut payload_json = serde_json::to_value(&req).unwrap_or(json!({}));

    state.sensitive.sanitize_json(&mut payload_json);
    let prompt_for_log = sanitize_text(&state.sensitive, &prompt);

    // Resolve model and think status for logging (and usage)
    let effective_think = false; // Disable think mode
    let config = llm_client::resolve_config()
    .map_err(|e| error_response(CODE_BAD_REQUEST, e).into_response())?;

    let request_id = begin_glm_request_log(
        &state.db,
        &client_ip,
        user_agent,
        "/expand/character",
        payload_json,
        &prompt_for_log,
        using_override_key,
        Some(&config.model),
        effective_think,
    )
    .await
    .map_err(|e| db_error_response(e).into_response())?;

    let db = state.db.clone();
    let sensitive = state.sensitive.clone();
    let req_clone = req.clone();

    let handle = tokio::spawn(async move {
        let start = std::time::Instant::now();
        
        let content = match llm_client::call_llm(
            prompt,
            true, // json_mode
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                let response_time_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                finish_glm_request_log(
                    &db,
                    request_id,
                    "failed",
                    None,
                    Some(&e),
                    Some(response_time_ms),
                )
                .await;
                return Err(error_response(CODE_INTERNAL_ERROR, e).into_response());
            }
        };

        let duration = start.elapsed();
        let response_time_ms = duration.as_millis().min(i64::MAX as u128) as i64;

        let clean = clean_json(&content);
        match serde_json::from_str::<Vec<CharacterInput>>(&clean) {
            Ok(chars) => {
                let chars_value = serde_json::to_value(&chars).unwrap_or(json!([]));
                // Log raw content as per user demand
                let chars_log = chars_value.to_string();

                finish_glm_request_log(
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
                finish_glm_request_log(
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
