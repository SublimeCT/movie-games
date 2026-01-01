use axum::serve;
use std::net::SocketAddr;

mod api_types;
mod app;
mod db;
mod glm;
mod handlers;
mod images;
mod prompt;
mod sensitive;
mod template;
#[cfg(test)]
mod tests_repro;
mod types;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let db_pool = db::init_pool()
        .await
        .expect("Failed to connect DATABASE_URL");
    db::init_db(&db_pool)
        .await
        .expect("Failed to init database");

    let sensitive = std::sync::Arc::new(sensitive::SensitiveFilter::from_env());

    let state = db::AppState {
        db: db_pool,
        sensitive,
    };
    let app = app::build_app(state);

    // 监听 0.0.0.0 以允许外部访问 (部署时的常见坑)
    // 端口已从 8080 改为 35275 (用户要求)
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "35275".to_string())
        .parse()
        .unwrap_or(35275);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
