use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;
use std::sync::OnceLock;

pub struct Config {
    pub server_addr: SocketAddr,
    pub redis_url: String,
    pub database_url: String,
}

impl Config {
    // Метод для инициализации конфигурации
    pub fn init() -> Self {
        // Загружаем .env файл, если он есть
        dotenv().ok();

        let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = env::var("SERVER_PORT").unwrap_or_else(|_| "3000".into());
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://user:password@localhost:5432/dnd_db".into());

        // Парсим адрес сразу здесь. Если конфиг кривой — лучше упасть на старте.
        let server_addr: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .expect("Failed to parse SERVER_HOST:SERVER_PORT");

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://default@localhost:6379".into());

        Self {
            server_addr,
            redis_url,
            database_url,
        }
    }
}

pub fn get_jwt_secret() -> &'static str {
    static JWT_SECRET: OnceLock<String> = OnceLock::new();
    JWT_SECRET.get_or_init(|| {
        dotenv().ok();
        env::var("JWT_SECRET").expect("JWT_SECRET must be set")
    })
}

pub fn get_auth_secret() -> &'static str {
    static AUTH_SECRET: OnceLock<String> = OnceLock::new();
    AUTH_SECRET.get_or_init(|| {
        dotenv().ok();
        env::var("AUTH_SECRET").expect("AUTH_SECRET must be set")
    })
}
