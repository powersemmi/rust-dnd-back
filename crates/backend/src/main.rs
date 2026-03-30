use axum::Router;
use axum::http::{HeaderValue, Method};
use axum::routing::{get, post};
use backend::handlers::{auth, room};
use backend::http_rate_limit;
use backend::{ApiDoc, Config, state};
use sqlx::postgres::PgPoolOptions;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

fn init_tracing(rust_log: &str) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(rust_log))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::init();
    init_tracing(&config.rust_log);

    let redis = redis::Client::open(config.redis_url.clone())?;
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = state::AppState::new(redis, pool);
    let allowed_origin = HeaderValue::from_str(&config.allowed_origin)?;

    let cors = CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ]);

    let router = Router::new()
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/ws/room", get(room::ws_room_handler))
        .route(
            "/api/auth/register",
            post(auth::register).route_layer(http_rate_limit::register_rate_limit_layer()),
        )
        .route(
            "/api/auth/login",
            post(auth::login).route_layer(http_rate_limit::login_rate_limit_layer()),
        )
        .route("/api/auth/refresh", post(auth::refresh_token))
        .route("/api/auth/me", get(auth::get_me))
        .layer(cors)
        .with_state(Arc::new(state));

    let addr: SocketAddr = SocketAddr::from((config.server_addr.ip(), config.server_addr.port()));
    tracing::info!("Listening on http://{}", addr);
    tracing::info!("Swagger run on http://{}/docs", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
