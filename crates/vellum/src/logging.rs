use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn log_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|home| home.join("Library").join("Logs").join("Vellum"))
            .unwrap_or_else(|| PathBuf::from("logs"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let xdg = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")));
        xdg.map(|p| p.join("vellum").join("logs"))
            .unwrap_or_else(|| PathBuf::from("logs"))
    }
}

fn default_log_level() -> &'static str {
    if cfg!(debug_assertions) {
        "vellum=debug,vellum_extension=debug,editor=debug,workspace=debug"
    } else {
        "vellum=info,vellum_extension=warn,editor=warn,workspace=warn"
    }
}

pub fn init() {
    let filter = EnvFilter::try_from_env("VELLUM_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new(default_log_level()));

    let log_path = log_dir();
    let log_path_display = log_path.display();

    let (file_appender, _guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily(&log_path, "vellum.log"),
    );

    let file_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_ansi(false);

    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .compact();

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer);

    if let Err(e) = subscriber.try_init() {
        eprintln!("failed to initialize logging: {}", e);
        return;
    }

    tracing::info!(
        log_dir = %log_path_display,
        version = env!("CARGO_PKG_VERSION"),
        "logging initialized"
    );

    Box::leak(Box::new(_guard));
}
