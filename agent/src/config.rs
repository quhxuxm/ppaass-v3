use accessory::Accessors;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Debug, Accessors)]
pub struct AgentConfig {
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
