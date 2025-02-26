use accessory::Accessors;
use ppaass_common::config::{
    DefaultConnectionPoolConfig, ProxyTcpConnectionConfig, ProxyTcpConnectionPoolConfig,
    ServerConfig,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Accessors, Debug)]
pub struct ProxyConfig {
    #[access(get(cp))]
    ip_v6: bool,
    #[access(get(cp))]
    server_port: u16,
    #[access(get(cp))]
    worker_thread_number: usize,
    #[access(get)]
    log_dir: PathBuf,
    #[access(get(ty=&str))]
    log_name_prefix: String,
    #[access(get(ty=&str))]
    max_log_level: String,
    #[access(get(ty=&std::path::Path))]
    user_dir: PathBuf,
    #[access(get(cp))]
    destination_connect_timeout: u64,
    #[access(get(cp))]
    agent_frame_buffer_size: usize,
    #[access(get(cp))]
    proxy_to_destination_data_relay_buffer_size: usize,
    #[access(get(cp))]
    destination_to_proxy_data_relay_buffer_size: usize,
    #[access(get)]
    forward: Option<ForwardConfig>,
    #[access(get(cp))]
    user_info_repository_refresh_interval: u64,
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

#[derive(Serialize, Deserialize, Accessors, Debug, Clone)]
pub struct ForwardConfig {
    proxy_connect_timeout: u64,
    #[access(get(ty=&std::path::Path))]
    user_dir: PathBuf,
    proxy_frame_buffer_size: usize,
    #[access(get)]
    connection_pool: Option<DefaultConnectionPoolConfig>,
    #[access(get)]
    username: Option<String>,
}

impl ProxyTcpConnectionConfig for ForwardConfig {
    fn proxy_frame_size(&self) -> usize {
        self.proxy_frame_buffer_size
    }
    fn proxy_connect_timeout(&self) -> u64 {
        self.proxy_connect_timeout
    }
}

impl ProxyTcpConnectionPoolConfig for ForwardConfig {
    fn max_pool_size(&self) -> usize {
        match &self.connection_pool {
            None => 0,
            Some(pool_config) => pool_config.max_pool_size(),
        }
    }
    fn fill_interval(&self) -> u64 {
        match &self.connection_pool {
            None => 0,
            Some(pool_config) => pool_config.fill_interval(),
        }
    }
    fn check_interval(&self) -> u64 {
        match &self.connection_pool {
            None => 0,
            Some(pool_config) => pool_config.check_interval(),
        }
    }
    fn connection_max_alive(&self) -> i64 {
        match &self.connection_pool {
            None => 0,
            Some(pool_config) => pool_config.connection_max_alive(),
        }
    }
    fn heartbeat_timeout(&self) -> u64 {
        match &self.connection_pool {
            None => 0,
            Some(pool_config) => pool_config.heartbeat_timeout(),
        }
    }
    fn retake_interval(&self) -> u64 {
        match &self.connection_pool {
            None => 2,
            Some(pool_config) => pool_config.retake_interval(),
        }
    }
}
