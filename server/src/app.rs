use axum::{
    http::Method,
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::db::AppState;
use crate::handlers::{expand_character, expand_worldview, generate, generate_prompt, hello};

pub(crate) fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    Router::new()
        .route("/", get(hello))
        .route("/generate", post(generate))
        .route("/generate/prompt", post(generate_prompt))
        .route("/expand/worldview", post(expand_worldview))
        .route("/expand/character", post(expand_character))
        .with_state(state)
        .layer(cors)
}
