use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub enum LogLevel {
    Error,
    Info,
    Warning,
    Debug,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectionPoolConfiguration {
    #[serde(rename = "checkInterval")]
    pub check_interval: u64,
    #[serde(rename = "fillInterval")]
    pub fill_interval: u64,
    #[serde(rename = "maxPoolSize")]
    pub max_pool_size: usize,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    #[serde(rename = "agentServerPort")]
    pub agent_server_port: u16,
    #[serde(rename = "workerThreadNumber")]
    pub worker_thread_number: usize,
    #[serde(rename = "maxLogLevel")]
    pub max_log_level: LogLevel,
    #[serde(rename = "connectionPoolConfiguration")]
    pub connection_pool_configuration: ConnectionPoolConfiguration,
}