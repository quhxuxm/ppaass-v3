use accessory::Accessors;
use ppaass_common::config::{
    ConnectionPoolConfig, RetrieveConnectionConfig, RetrieveConnectionPoolConfig,
    RetrieveServerConfig,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Debug, Accessors)]
pub struct AgentConfig {
    pub ip_v6: bool,
    pub server_port: u16,
    pub worker_thread_number: usize,
    pub username: String,
    pub log_dir: PathBuf,
    pub log_name_prefix: String,
    pub max_log_level: String,
    pub user_dir: PathBuf,
    pub agent_to_proxy_data_relay_buffer_size: usize,
    pub proxy_to_agent_data_relay_buffer_size: usize,
    pub proxy_frame_buffer_size: usize,
    pub proxy_connect_timeout: u64,
    pub user_info_repository_refresh_interval: u64,
    pub connection_pool: Option<ConnectionPoolConfig>,
}

impl RetrieveConnectionConfig for AgentConfig {
    fn frame_size(&self) -> usize {
        self.proxy_frame_buffer_size
    }
    fn connect_timeout(&self) -> u64 {
        self.proxy_connect_timeout
    }
}

impl RetrieveConnectionPoolConfig for AgentConfig {
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
    fn retake_interval(&self) -> u64 {
        match self.connection_pool {
            Some(ref pool) => pool.retake_interval(),
            None => 2,
        }
    }
}

impl RetrieveServerConfig for AgentConfig {
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
