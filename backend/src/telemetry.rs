use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::AppEnv;

pub fn init(app_env: AppEnv) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tj_shop=debug,sqlx=warn,tower_http=info"));

    let registry = tracing_subscriber::registry().with(filter);

    if app_env.is_dev() {
        registry
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    }
}
