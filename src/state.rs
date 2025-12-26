use redis::Client;

pub struct AppState {
    redis: redis::Client,
}

impl AppState {
    pub fn new(client: Client) -> Self {
        Self { redis: client }
    }

    pub fn get_redis(&self) -> &Client {
        &self.redis
    }
}
