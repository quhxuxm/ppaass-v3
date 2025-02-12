use accessory::Accessors;
use ppaass_common::config::{
    ConnectionPoolConfig, ProxyTcpConnectionConfig, ServerConfig, UserInfoConfig,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Debug, Accessors)]
pub struct AgentConfig {
    ip_v6: bool,
    server_port: u16,
    worker_thread_number: usize,
    #[access(get)]
    log_dir: PathBuf,
    #[access(get(ty=&str))]
    log_name_prefix: String,
    #[access(get(ty=&str))]
    max_log_level: String,
    #[access(get(ty=&std::path::Path))]
    user_dir: PathBuf,
    username: String,
    #[access(get(cp))]
    agent_to_proxy_data_relay_buffer_size: usize,
    #[access(get(cp))]
    proxy_to_agent_data_relay_buffer_size: usize,

    proxy_frame_buffer_size: usize,

    proxy_connect_timeout: u64,
    #[access(get)]
    connection_pool: Option<ConnectionPoolConfig>,
}
impl UserInfoConfig for AgentConfig {
    fn username(&self) -> &str {
        self.username.as_str()
    }
}

impl ProxyTcpConnectionConfig for AgentConfig {
    fn proxy_frame_size(&self) -> usize {
        self.proxy_frame_buffer_size
    }
    fn proxy_connect_timeout(&self) -> u64 {
        self.proxy_connect_timeout
    }
}

impl ServerConfig for AgentConfig {
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
