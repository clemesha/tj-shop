use std::net::SocketAddr;

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Dev,
    Prod,
}

impl AppEnv {
    pub fn is_dev(self) -> bool {
        matches!(self, AppEnv::Dev)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    pub db_pool_max: u32,
    pub app_env: AppEnv,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL is required")?;

        let listen_addr = std::env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
            .parse::<SocketAddr>()
            .context("LISTEN_ADDR must be a valid socket address")?;

        let db_pool_max = std::env::var("DB_POOL_MAX")
            .ok()
            .map(|s| s.parse::<u32>())
            .transpose()
            .context("DB_POOL_MAX must be a positive integer")?
            .unwrap_or(10);

        let app_env = match std::env::var("APP_ENV")
            .unwrap_or_else(|_| "dev".to_string())
            .as_str()
        {
            "prod" | "production" => AppEnv::Prod,
            _ => AppEnv::Dev,
        };

        Ok(Self {
            database_url,
            listen_addr,
            db_pool_max,
            app_env,
        })
    }
}
