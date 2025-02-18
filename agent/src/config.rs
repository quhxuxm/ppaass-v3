use accessory::Accessors;
use ppaass_common::config::{
    DefaultConnectionPoolConfig, ProxyTcpConnectionConfig, ProxyTcpConnectionPoolConfig,
    ServerConfig,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Accessors)]
pub struct AgentConfig {
    ip_v6: bool,
    server_port: u16,
    worker_thread_number: usize,
    #[access(get)]
    username: Option<String>,
    #[access(get)]
    log_dir: PathBuf,
    #[access(get(ty=&str))]
    log_name_prefix: String,
    #[access(get(ty=&str))]
    max_log_level: String,
    #[access(get(ty=&std::path::Path))]
    user_dir: PathBuf,
    #[access(get(cp))]
    agent_to_proxy_data_relay_buffer_size: usize,
    #[access(get(cp))]
    proxy_to_agent_data_relay_buffer_size: usize,
    proxy_frame_buffer_size: usize,
    proxy_connect_timeout: u64,
    #[access(get(cp))]
    user_info_repository_refresh_interval: u64,
    #[access(get)]
    connection_pool: Option<DefaultConnectionPoolConfig>,
}

impl ProxyTcpConnectionConfig for AgentConfig {
    fn proxy_frame_size(&self) -> usize {
        self.proxy_frame_buffer_size
    }
    fn proxy_connect_timeout(&self) -> u64 {
        self.proxy_connect_timeout
    }
}

impl ProxyTcpConnectionPoolConfig for AgentConfig {
    fn max_pool_size(&self) -> usize {
        match self.connection_pool {
            Some(ref pool) => pool.max_pool_size(),
            None => 0,
        }
    }
    fn fill_interval(&self) -> u64 {
        match self.connection_pool {
            Some(ref pool) => pool.fill_interval(),
            None => 0,
        }
    }
    fn check_interval(&self) -> u64 {
        match self.connection_pool {
            Some(ref pool) => pool.check_interval(),
            None => 0,
        }
    }
    fn connection_max_alive(&self) -> i64 {
        match self.connection_pool {
            Some(ref pool) => pool.connection_max_alive(),
            None => 0,
        }
    }
    fn heartbeat_timeout(&self) -> u64 {
        match self.connection_pool {
            Some(ref pool) => pool.heartbeat_timeout(),
            None => 0,
        }
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
