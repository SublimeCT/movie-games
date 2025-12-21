use axum::{http::StatusCode, Json};
use serde_json::json;
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

pub(crate) async fn begin_glm_request_log(
    db: &PgPool,
    client_ip: &str,
    route: &str,
    request_payload: serde_json::Value,
    glm_prompt: &str,
    using_override_key: bool,
) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    let mut tx = db.begin().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "DB Error" })),
        )
    })?;

    let _ = sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(9001i64)
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "DB Error" })),
            )
        })?;

    // Check daily limit (30 requests per IP per day)
    let daily_count: i64 = sqlx::query_scalar(
        "select count(*) from glm_requests where client_ip = $1 and created_at > current_date",
    )
    .bind(client_ip)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "DB Error" })),
        )
    })?;

    if daily_count >= 30 && !using_override_key {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "今日免费额度已用完 (30次/天)，请填写 API Key 继续使用",
                "code": "API_KEY_REQUIRED_DAILY_LIMIT",
                "dailyRequests": daily_count,
            })),
        ));
    }

    let active: i64 = sqlx::query_scalar(
        "select count(*) from glm_requests where status = 'running' and created_at > now() - interval '10 minutes'",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "DB Error" }))))?;

    if active >= 2 && !using_override_key {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "当前并发较高，请填写 API Key 后重试",
                "code": "API_KEY_REQUIRED",
                "activeRequests": active,
            })),
        ));
    }

    let id = Uuid::new_v4();
    sqlx::query(
        "insert into glm_requests (id, client_ip, route, status, request_payload, glm_prompt) values ($1, $2, $3, 'running', $4, $5)",
    )
    .bind(id)
    .bind(client_ip)
    .bind(route)
    .bind(request_payload)
    .bind(glm_prompt)
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "DB Error" }))))?;

    tx.commit().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "DB Error" })),
        )
    })?;

    Ok(id)
}

pub(crate) async fn finish_glm_request_log(
    db: &PgPool,
    id: Uuid,
    status: &str,
    response: Option<&str>,
    error: Option<&str>,
    response_time_ms: Option<i64>,
) {
    let _ = sqlx::query(
        "update glm_requests set status = $2, glm_response = $3, error_text = $4, response_time_ms = $5, updated_at = now() where id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(response)
    .bind(error)
    .bind(response_time_ms)
    .execute(db)
    .await;
}
