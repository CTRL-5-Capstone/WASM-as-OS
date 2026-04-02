use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing(log_level: &str, log_format: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    let registry = tracing_subscriber::registry().with(env_filter);

    match log_format {
        "json" => {
            registry
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        _ => {
            registry
                .with(tracing_subscriber::fmt::layer().pretty())
                .init();
        }
    }

    tracing::info!("Logging initialized with level: {}, format: {}", log_level, log_format);
}
