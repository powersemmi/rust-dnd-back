use axum::Router;
use axum::routing::get;
use backend::{ApiDoc, Config, handler, state};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

fn init_tracing() {
    // Настройка логирования
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "dnd_back=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();
    let config = Config::init();
    let redis = redis::Client::open(config.redis_url).unwrap();
    let state = state::AppState::new(redis);

    let router = Router::new()
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/ws/room", get(handler::ws_handler))
        .with_state(Arc::from(state));

    let addr: SocketAddr = SocketAddr::from((config.server_addr.ip(), config.server_addr.port()));
    tracing::info!("Listening on http://{}", addr);
    tracing::info!("Swagger run on http://{}/docs", addr);

    let listener: tokio::net::TcpListener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
