use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: PgPool,
}

pub(crate) async fn init_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("MOVIE_GAMES_DATABASE_URL").expect("MOVIE_GAMES_DATABASE_URL is required");
    PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await
}

pub(crate) async fn init_db(db: &PgPool) -> Result<(), sqlx::Error> {
    // 运行数据库迁移
    // 注意：在生产环境，建议通过 sqlx cli 或 CI/CD 流程执行迁移，
    // 但为了简化部署，这里保留了自动迁移功能。
    // 如果用户希望手动执行，可以注释掉此行，并手动运行 migrations 目录下的 SQL。
    sqlx::migrate!("./migrations")
        .run(db)
        .await?;

    Ok(())
}

// 数据库错误类型 - 用于与 handlers.rs 中的 ApiResponse 兼容
#[derive(Debug)]
pub(crate) enum DbError {
    DailyLimitExceeded,
    TooManyRequests,
    // InvalidBaseUrl, // Unused
    InternalError,
}

impl DbError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            DbError::DailyLimitExceeded => "API_KEY_REQUIRED_DAILY_LIMIT",
            DbError::TooManyRequests => "API_KEY_REQUIRED",
            // DbError::InvalidBaseUrl => "INVALID_BASE_URL",
            DbError::InternalError => "INTERNAL_ERROR",
        }
    }

    pub(crate) fn message(&self) -> &'static str {
        match self {
            DbError::DailyLimitExceeded => "今日免费额度已用完 (30次/天)，请填写 API Key 继续使用",
            DbError::TooManyRequests => "当前并发较高，请填写 API Key 后重试",
            // DbError::InvalidBaseUrl => "Invalid baseUrl",
            DbError::InternalError => "DB Error",
        }
    }
}

pub(crate) async fn begin_glm_request_log(
    db: &PgPool,
    client_ip: &str,
    user_agent: &str,
    route: &str,
    request_payload: serde_json::Value,
    glm_prompt: &str,
    using_override_key: bool,
) -> Result<Uuid, DbError> {
    let mut tx = db.begin().await.map_err(|_| DbError::InternalError)?;

    let _ = sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(9001i64)
        .execute(&mut *tx)
        .await
        .map_err(|_| DbError::InternalError)?;

    // Check daily limit (30 requests per IP per day)
    let daily_count: i64 = sqlx::query_scalar(
        "select count(*) from glm_requests where client_ip = $1 and created_at > current_date",
    )
    .bind(client_ip)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| DbError::InternalError)?;

    if daily_count >= 30 && !using_override_key {
        return Err(DbError::DailyLimitExceeded);
    }

    // Check recent request frequency (2 requests per 5 minutes per IP)
    // Only applies if not using own API Key
    let active: i64 = sqlx::query_scalar(
        "select count(*) from glm_requests where client_ip = $1 and created_at > now() - interval '5 minutes'",
    )
    .bind(client_ip)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| DbError::InternalError)?;

    if active >= 2 && !using_override_key {
        return Err(DbError::TooManyRequests);
    }

    let id = Uuid::new_v4();
    sqlx::query(
        "insert into glm_requests (id, client_ip, user_agent, route, status, request_payload, glm_prompt) values ($1, $2, $3, $4, 'running', $5, $6)",
    )
    .bind(id)
    .bind(client_ip)
    .bind(user_agent)
    .bind(route)
    .bind(request_payload)
    .bind(glm_prompt)
    .execute(&mut *tx)
    .await
    .map_err(|_| DbError::InternalError)?;

    tx.commit().await.map_err(|_| DbError::InternalError)?;

    Ok(id)
}

pub(crate) async fn finish_glm_request_log(
    db: &PgPool,
    id: Uuid,
    status: &str,
    response_content: Option<&str>,
    error_message: Option<&str>,
    response_time_ms: Option<i64>,
) {
    let result = sqlx::query(
        "update glm_requests set status = $1, glm_response = $2, error_text = $3, response_time_ms = $4, updated_at = now() where id = $5",
    )
    .bind(status)
    .bind(response_content)
    .bind(error_message)
    .bind(response_time_ms)
    .bind(id)
    .execute(db)
    .await;

    if let Err(e) = result {
        eprintln!("Failed to update glm_request log: {}", e);
    }
}

pub(crate) async fn save_processed_response(
    db: &PgPool,
    id: Uuid,
    response: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("update glm_requests set processed_response = $1 where id = $2")
        .bind(response)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub(crate) async fn get_request_owner(
    db: &PgPool,
    id: Uuid,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row: Option<(String, String)> = sqlx::query_as("select client_ip, status from glm_requests where id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(row)
}

pub(crate) async fn set_share_status(
    db: &PgPool,
    id: Uuid,
    shared: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("update glm_requests set shared = $1 where id = $2")
        .bind(shared)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub(crate) async fn get_shared_game(
    db: &PgPool,
    id: Uuid,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as("select processed_response from glm_requests where id = $1 and shared = true")
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|r| r.0))
}

pub(crate) async fn record_visit(
    db: &PgPool,
    request_id: Uuid,
    client_ip: &str,
    user_agent: &str,
    referer: Option<&str>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("insert into records (id, request_id, client_ip, user_agent, referer) values ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(request_id)
        .bind(client_ip)
        .bind(user_agent)
        .bind(referer)
        .execute(db)
        .await?;
    Ok(())
}
