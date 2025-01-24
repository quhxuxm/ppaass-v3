use accessory::Accessors;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Accessors)]
pub struct ServerConfig {
    #[access(get)]
    ip_v6: bool,
    #[access(get)]
    port: u16,
    #[access(get)]
    worker_threads: usize,
    #[access(get)]
    log_dir: PathBuf,
    #[access(get(ty=&str))]
    log_name_prefix: String,
    #[access(get(ty=&str))]
    max_log_level: String,
    #[access(get)]
    rsa_dir: PathBuf,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 80,
            worker_threads: 32,
            ip_v6: false,
            log_dir: PathBuf::from("log"),
            log_name_prefix: "ppaass-v3-server".to_string(),
            max_log_level: "info".to_string(),
            rsa_dir: PathBuf::from("rsa"),
        }
    }
}
