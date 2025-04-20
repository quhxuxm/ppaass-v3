use chrono::{DateTime, Local};
#[derive(Debug)]
pub struct UploadSpeedEvent {
    pub speed: u64,
}

#[derive(Debug)]
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
