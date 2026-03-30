use redis::Client;
use sqlx::PgPool;

pub struct AppState {
    pub redis: Client,
    pub pg_pool: PgPool,
}

impl AppState {
    pub fn new(redis: Client, pg_pool: PgPool) -> Self {
        Self { redis, pg_pool }
    }
}
