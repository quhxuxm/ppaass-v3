use accessory::Accessors;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Accessors)]
pub struct ServerConfig {
    #[access(get)]
    ip_v6: bool,
    #[access(get)]
    port: u16,
    #[access(get)]
    worker_threads: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 80,
            worker_threads: 32,
            ip_v6: false,
        }
    }
}
