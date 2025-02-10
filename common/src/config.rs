use crate::ProxyTcpConnectionPoolConfig;
use serde::{Deserialize, Serialize};

pub trait ServerConfig {
    fn worker_thread_number(&self) -> usize;
    fn server_port(&self) -> u16;
    fn ip_v6(&self) -> bool;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionPoolConfig {
    max_pool_size: usize,
    fill_interval: u64,
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
