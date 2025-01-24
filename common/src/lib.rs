pub mod crypto;
pub mod error;
use crate::error::CommonError;
use rand::random;
use std::path::Path;
use std::str::FromStr;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::ChronoUtc;
use uuid::Uuid;
/// Generate a random UUID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}

/// Generate a random 32 bytes vector
pub fn random_32_bytes() -> Vec<u8> {
    let random_32_bytes = random::<[u8; 32]>();
    random_32_bytes.to_vec()
}

/// Init the logger
pub fn init_logger(
    // The folder to store the log file
    log_folder: &Path,
    // The log name prefix
    log_name_prefix: &str,
    // The max log level
    max_log_level: &str,
) -> Result<WorkerGuard, CommonError> {
    let (trace_file_appender, _trace_appender_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily(log_folder, log_name_prefix),
    );
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(max_log_level)?)
        .with_writer(trace_file_appender)
        .with_line_number(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_timer(ChronoUtc::rfc_3339())
        .with_ansi(false)
        .init();
    Ok(_trace_appender_guard)
}
