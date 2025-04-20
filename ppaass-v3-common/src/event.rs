use chrono::{DateTime, Local};
pub struct UploadSpeedEvent {
    pub speed: u64,
}
pub struct DownloadSpeedEvent {
    pub speed: u64,
}

#[derive(Debug)]
pub enum LogEventLevel {
    Error,
    Info,
    Warning,
    Debug,
    Trace,
}

#[derive(Debug)]
pub struct LogEvent {
    pub level: LogEventLevel,
    pub timestamp: DateTime<Local>,
    pub message: String,
}
