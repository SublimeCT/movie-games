use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::sensitive::SensitiveFilter;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: PgPool,
    pub(crate) sensitive: Arc<SensitiveFilter>,
}

pub(crate) async fn init_pool() -> Result<PgPool, sqlx::Error> {
    let database_url =
        std::env::var("MOVIE_GAMES_DATABASE_URL").expect("MOVIE_GAMES_DATABASE_URL is required");
    PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await
}

pub(crate) async fn init_db(db: &PgPool) -> Result<(), sqlx::Error> {
    let result = sqlx::migrate!("./migrations").run(db).await;
    match result {
        Ok(()) => Ok(()),
        Err(e) => match e {
            sqlx::migrate::MigrateError::VersionMismatch(version) => {
                let allow = std::env::var("MOVIE_GAMES_ALLOW_MIGRATE_VERSION_MISMATCH")
                    .unwrap_or_else(|_| "1".to_string());
                if allow.trim() == "1" {
                    eprintln!(
                        "Database migration version mismatch detected ({}). Continuing because MOVIE_GAMES_ALLOW_MIGRATE_VERSION_MISMATCH=1.",
                        version
                    );
                    Ok(())
                } else {
                    Err(sqlx::Error::Migrate(Box::new(e)))
                }
            }
            _ => Err(sqlx::Error::Migrate(Box::new(e))),
        },
    }
}

// 数据库错误类型 - 用于与 handlers.rs 中的 ApiResponse 兼容
#[derive(Debug)]
pub(crate) enum DbError {
    DailyLimitExceeded,
    TooManyRequests,
    ServiceBusy,
    // InvalidBaseUrl, // Unused
    InternalError,
}

impl DbError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            DbError::DailyLimitExceeded => "API_KEY_REQUIRED_DAILY_LIMIT",
            DbError::TooManyRequests => "API_KEY_REQUIRED",
            DbError::ServiceBusy => "SERVICE_BUSY",
            // DbError::InvalidBaseUrl => "INVALID_BASE_URL",
            DbError::InternalError => "INTERNAL_ERROR",
        }
    }

    pub(crate) fn message(&self) -> &'static str {
        match self {
            DbError::DailyLimitExceeded => "今日免费额度已用完 (30次/天)，请填写 API Key 继续使用",
            DbError::TooManyRequests => "当前并发较高，请填写 API Key 后重试",
            DbError::ServiceBusy => "服务繁忙",
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

    if route == "/generate" {
        let daily_total: i64 = sqlx::query_scalar(
            "select count(*) from glm_requests where route = $1 and created_at > current_date",
        )
        .bind(route)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| DbError::InternalError)?;

        if daily_total >= 60 {
            return Err(DbError::ServiceBusy);
        }
    }

    // Check daily limit (30 requests per IP per day) - only applies if not using own API Key
    let daily_count: i64 = sqlx::query_scalar(
        "select count(*) from glm_requests where client_ip = $1 and route = $2 and created_at > current_date",
    )
    .bind(client_ip)
    .bind(route)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| DbError::InternalError)?;

    if daily_count >= 30 && !using_override_key {
        return Err(DbError::DailyLimitExceeded);
    }

    // Check recent request frequency (2 requests per 5 minutes per IP)
    // Only applies if not using own API Key
    let active: i64 = sqlx::query_scalar(
        "select count(*) from glm_requests where client_ip = $1 and route = $2 and created_at > now() - interval '5 minutes'",
    )
    .bind(client_ip)
    .bind(route)
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
    let row: Option<(String, String)> =
        sqlx::query_as("select client_ip, status from glm_requests where id = $1")
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

pub(crate) async fn delete_game_by_request_id(db: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;

    sqlx::query("delete from records where request_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("delete from shared_records where request_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("delete from glm_requests where id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub(crate) async fn get_game_for_play(
    db: &PgPool,
    id: Uuid,
) -> Result<Option<(serde_json::Value, bool, String)>, sqlx::Error> {
    let row: Option<(serde_json::Value, bool, String)> = sqlx::query_as(
        "select processed_response, shared, client_ip from glm_requests where id = $1 and status = 'success'",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;

    Ok(row)
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

pub(crate) async fn upsert_shared_record(
    db: &PgPool,
    request_id: Uuid,
    shared_ip: &str,
    shared_user_agent: Option<&str>,
) -> Result<Uuid, DbError> {
    let mut tx = db.begin().await.map_err(|_| DbError::InternalError)?;

    let _ = sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(9002i64)
        .execute(&mut *tx)
        .await
        .map_err(|_| DbError::InternalError)?;

    let existing: Option<Uuid> =
        sqlx::query_scalar("select id from shared_records where request_id = $1")
            .bind(request_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| DbError::InternalError)?;

    if let Some(id) = existing {
        sqlx::query(
            "update shared_records set shared_ip = $2, shared_user_agent = $3 where request_id = $1",
        )
        .bind(request_id)
        .bind(shared_ip)
        .bind(shared_user_agent)
        .execute(&mut *tx)
        .await
        .map_err(|_| DbError::InternalError)?;

        tx.commit().await.map_err(|_| DbError::InternalError)?;
        return Ok(id);
    }

    let daily_total: i64 =
        sqlx::query_scalar("select count(*) from shared_records where shared_at > current_date")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| DbError::InternalError)?;

    if daily_total >= 20 {
        return Err(DbError::ServiceBusy);
    }

    let daily_ip: i64 = sqlx::query_scalar(
        "select count(*) from shared_records where shared_ip = $1 and shared_at > current_date",
    )
    .bind(shared_ip)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| DbError::InternalError)?;

    if daily_ip >= 3 {
        return Err(DbError::ServiceBusy);
    }

    let id = Uuid::new_v4();
    let row: (Uuid,) = sqlx::query_as(
        "insert into shared_records (id, request_id, shared_ip, shared_user_agent) values ($1, $2, $3, $4) returning id",
    )
    .bind(id)
    .bind(request_id)
    .bind(shared_ip)
    .bind(shared_user_agent)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| DbError::InternalError)?;

    tx.commit().await.map_err(|_| DbError::InternalError)?;

    Ok(row.0)
}

pub(crate) async fn get_shared_record_id_by_request_id(
    db: &PgPool,
    request_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    let row: Option<(Uuid,)> =
        sqlx::query_as("select id from shared_records where request_id = $1")
            .bind(request_id)
            .fetch_optional(db)
            .await?;
    Ok(row.map(|r| r.0))
}

pub(crate) async fn get_shared_record_meta_by_request_id(
    db: &PgPool,
    request_id: Uuid,
) -> Result<Option<(Option<Uuid>, bool, Option<String>, String)>, sqlx::Error> {
    let row: Option<(Option<Uuid>, bool, Option<String>, String)> = sqlx::query_as(
        "select sr.id, gr.shared, sr.shared_at::text, gr.client_ip \
         from glm_requests gr \
         left join shared_records sr on sr.request_id = gr.id \
         where gr.id = $1",
    )
    .bind(request_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

pub(crate) async fn list_shared_records_by_ids(
    db: &PgPool,
    ids: &[Uuid],
    owner_ip: &str,
) -> Result<
    Vec<(
        Uuid,
        Uuid,
        String,
        bool,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    )>,
    sqlx::Error,
> {
    let rows = sqlx::query_as(
        "select \
            sr.id, \
            sr.request_id, \
            sr.shared_at::text, \
            gr.shared, \
            (gr.processed_response->>'title') as title, \
            (gr.processed_response->'meta'->>'synopsis') as synopsis, \
            (gr.processed_response->'meta'->>'genre') as genre, \
            (gr.processed_response->'meta'->>'language') as language, \
            (select count(*) from records r where r.request_id = sr.request_id) as play_count \
         from shared_records sr \
         join glm_requests gr on gr.id = sr.request_id \
         where sr.id = any($1) \
           and (
             gr.client_ip = $2
             or ($2 = '::1' and gr.client_ip = '127.0.0.1')
             or ($2 = '127.0.0.1' and gr.client_ip = '::1')
           ) \
         order by sr.shared_at desc",
    )
    .bind(ids)
    .bind(owner_ip)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

pub(crate) async fn create_imported_request(
    db: &PgPool,
    client_ip: &str,
    user_agent: &str,
    request_payload: serde_json::Value,
    processed_response: serde_json::Value,
) -> Result<Uuid, DbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "insert into glm_requests (id, client_ip, user_agent, route, status, request_payload, glm_prompt, processed_response, template_source) values ($1, $2, $3, '/import', 'success', $4, '[import]', $5, 'import')",
    )
    .bind(id)
    .bind(client_ip)
    .bind(user_agent)
    .bind(request_payload)
    .bind(processed_response)
    .execute(db)
    .await
    .map_err(|_| DbError::InternalError)?;

    Ok(id)
}

pub(crate) async fn set_request_template_source(
    db: &PgPool,
    id: Uuid,
    source: &str,
) -> Result<(), DbError> {
    sqlx::query("update glm_requests set template_source = $1, updated_at = now() where id = $2")
        .bind(source)
        .bind(id)
        .execute(db)
        .await
        .map_err(|_| DbError::InternalError)?;
    Ok(())
}
