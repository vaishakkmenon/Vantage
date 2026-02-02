// src/logging.rs
use std::{path::Path, sync::OnceLock};
use tracing_subscriber::{EnvFilter, fmt};

static GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();
static INIT: OnceLock<()> = OnceLock::new();

/// Initialize logging once for the whole process.
/// - `path`: e.g., "logs/perft.log"
/// - `filter`: e.g., "perft=trace,execute=debug"
pub fn init_logging<P: AsRef<Path>>(path: P, filter: &str) {
    INIT.get_or_init(|| {
        let path = path.as_ref();

        // Ensure directory exists
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        // Non-blocking writer to a single file (simple & fast for tests)
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open log file");

        let (non_blocking, guard) = tracing_appender::non_blocking(file);
        // Keep the guard alive for the program lifetime
        let _ = GUARD.set(guard);

        // Allow runtime filtering like: RUST_LOG="perft=trace,execute=debug"
        // but also accept a string argument for convenience in tests.
        let env_filter = if std::env::var_os("RUST_LOG").is_some() {
            EnvFilter::from_default_env()
        } else {
            EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("info"))
        };

        let subscriber = fmt()
            .with_env_filter(env_filter)
            .with_ansi(false) // plain text in files
            .with_target(true) // show module e.g. moves::execute
            .with_file(true) // include filename
            .with_line_number(true) // include line number
            .with_writer(non_blocking)
            .finish();

        // Ignore error if someone already set a global subscriber (idempotent for tests)
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}
