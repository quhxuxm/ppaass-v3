use serde::{Deserialize, Serialize};
pub trait RetrieveConnectionPoolConfig {
    fn max_pool_size(&self) -> usize;
    fn fill_interval(&self) -> u64;
    fn check_interval(&self) -> u64;
    fn connection_max_alive(&self) -> i64;
    fn heartbeat_timeout(&self) -> u64;
    fn retake_interval(&self) -> u64;
}

pub trait RetrieveConnectionConfig {
    fn frame_size(&self) -> usize;
    fn connect_timeout(&self) -> u64;
}

pub trait RetrieveServerConfig {
    fn worker_thread_number(&self) -> usize;
    fn server_port(&self) -> u16;
    fn ip_v6(&self) -> bool;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub max_pool_size: usize,
    pub fill_interval: u64,
    pub check_interval: u64,
    pub connection_max_alive: i64,
    pub heartbeat_timeout: u64,
    pub retake_interval: u64,
}

impl RetrieveConnectionPoolConfig for ConnectionPoolConfig {
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
    fn retake_interval(&self) -> u64 {
        self.retake_interval
    }
}
