use accessory::Accessors;
use ppaass_common::config::ServerConfig;
use ppaass_common::error::CommonError;
use ppaass_common::{
    parse_to_socket_addresses, ProxyTcpConnectionInfo, ProxyTcpConnectionInfoSelector,
    ProxyTcpConnectionPoolConfig,
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
    #[access(get)]
    rsa_dir: PathBuf,
    #[access(get)]
    proxy_addresses: Vec<String>,
    #[access(get(ty=&str))]
    authentication: String,
    #[access(get(cp))]
    agent_to_proxy_data_relay_buffer_size: usize,
    #[access(get(cp))]
    proxy_to_agent_data_relay_buffer_size: usize,
    #[access(get(cp))]
    proxy_framed_buffer_size: usize,
    #[access(get(cp))]
    proxy_connect_timeout: u64,
    #[access(get)]
    connection_pool: Option<ConnectionPoolConfig>,
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

#[derive(Serialize, Deserialize, Debug, Accessors, Clone)]
pub struct ConnectionPoolConfig {
    max_pool_size: usize,
    fill_interval: u64,
    connection_retake_interval: u64,
    check_interval: u64,
    connection_max_alive: i64,
    heartbeat_timeout: u64,
}

impl ProxyTcpConnectionPoolConfig for ConnectionPoolConfig {
    fn max_pool_size(&self) -> usize {
        self.max_pool_size
    }
    fn fill_interval(&self) -> u64 {
        self.fill_interval
    }
    fn connection_retake_interval(&self) -> u64 {
        self.connection_retake_interval
    }
    fn check_interval(&self) -> u64 {
        self.check_interval
    }
    fn connection_max_alive(&self) -> i64 {
        self.connection_max_alive
    }
    fn heartbeat_timeout(&self) -> u64 {
        self.heartbeat_timeout
    }
}

impl ProxyTcpConnectionInfoSelector for AgentConfig {
    fn select_proxy_tcp_connection_info(&self) -> Result<ProxyTcpConnectionInfo, CommonError> {
        Ok(ProxyTcpConnectionInfo::new(
            parse_to_socket_addresses(self.proxy_addresses.iter())?,
            self.authentication.clone(),
            self.proxy_framed_buffer_size(),
            self.proxy_connect_timeout(),
        ))
    }
}
