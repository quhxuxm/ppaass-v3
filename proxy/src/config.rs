use accessory::Accessors;
use ppaass_common::config::ServerConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Accessors)]
pub struct ProxyConfig {
    #[access(get)]
    ip_v6: bool,
    #[access(get)]
    server_port: u16,
    #[access(get)]
    worker_thread_number: usize,
    #[access(get)]
    log_dir: PathBuf,
    #[access(get(ty=&str))]
    log_name_prefix: String,
    #[access(get(ty=&str))]
    max_log_level: String,
    #[access(get)]
    rsa_dir: PathBuf,
}

impl ServerConfig for ProxyConfig {
    fn worker_thread_number(&self) -> usize {
        self.worker_thread_number
    }
    fn server_port(&self) -> u16 {
        self.server_port
    }
    fn ip_v6(&self) -> bool {
        self.ip_v6
    }
}
impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            server_port: 80,
            worker_thread_number: 32,
            ip_v6: false,
            log_dir: PathBuf::from("log"),
            log_name_prefix: "ppaass-v3-proxy".to_string(),
            max_log_level: "info".to_string(),
            rsa_dir: PathBuf::from("rsa"),
        }
    }
}
