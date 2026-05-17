use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tokio::signal;

use tj_shop::{AppState, config::Config, db, router, telemetry};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let config = Config::from_env()?;
    telemetry::init(config.app_env);

    tracing::info!(
        app_env = ?config.app_env,
        listen_addr = %config.listen_addr,
        "starting tj-shop"
    );

    let db = db::connect(&config).await?;
    db::migrate(&db).await?;

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
    };

    let listener = TcpListener::bind(config.listen_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.listen_addr))?;

    tracing::info!(addr = %config.listen_addr, "listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install ctrl_c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
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

    tracing::info!("shutdown signal received");
}
