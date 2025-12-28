use redis::Client;
use sqlx::PgPool;

pub struct AppState {
    redis: Client,
    pg_pool: PgPool,
}

impl AppState {
    pub fn new(redis: Client, pg_pool: PgPool) -> Self {
        Self { redis, pg_pool }
    }
    pub fn get_redis(&self) -> &Client {
        &self.redis
    }
    pub fn get_pg_pool(&self) -> &PgPool {
        &self.pg_pool
    }
}
