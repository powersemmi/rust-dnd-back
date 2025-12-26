use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;

pub struct Config {
    pub server_addr: SocketAddr,
    pub redis_url: String,
}

impl Config {
    // Метод для инициализации конфигурации
    pub fn init() -> Self {
        // Загружаем .env файл, если он есть
        dotenv().ok();

        let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = env::var("SERVER_PORT").unwrap_or_else(|_| "3000".into());

        // Парсим адрес сразу здесь. Если конфиг кривой — лучше упасть на старте.
        let server_addr: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .expect("Failed to parse SERVER_HOST:SERVER_PORT");

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://default@localhost:6379".into());

        Self {
            server_addr,
            redis_url,
        }
    }
}
