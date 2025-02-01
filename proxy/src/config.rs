use accessory::Accessors;
use ppaass_common::config::ServerConfig;
use ppaass_common::error::CommonError;
use ppaass_common::{
    parse_to_socket_addresses, ProxyTcpConnectionInfo, ProxyTcpConnectionInfoSelector,
    ProxyTcpConnectionPoolConfig,
};
use rand::random;
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
    #[access(get)]
    agent_rsa_dir: PathBuf,
    #[access(get(cp))]
    destination_connect_timeout: u64,
    #[access(get(cp))]
    forward_proxy_connect_timeout: Option<u64>,
    #[access(get)]
    forward_proxies: Option<Vec<ForwardProxyInfo>>,
    #[access(get)]
    forward_rsa_dir: Option<PathBuf>,
    forward_max_pool_size: Option<usize>,
    forward_fill_interval: Option<u64>,
    forward_connection_retake_interval: Option<u64>,
    forward_check_interval: Option<u64>,
    forward_connection_max_alive: Option<i64>,
    forward_heartbeat_timeout: Option<u64>,
    #[access(get(cp))]
    proxy_to_destination_data_relay_buffer_size: usize,
    #[access(get(cp))]
    destination_to_proxy_data_relay_buffer_size: usize,
    #[access(get(cp))]
    forward_proxy_framed_buffer_size: Option<usize>,
}

#[derive(Serialize, Deserialize, Accessors, Debug)]
pub struct ForwardProxyInfo {
    #[access(get(ty=&str))]
    pub proxy_address: String,
    #[access(get(ty=&str))]
    pub proxy_auth: String,
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

impl ProxyTcpConnectionPoolConfig for ProxyConfig {
    fn max_pool_size(&self) -> usize {
        self.forward_max_pool_size.unwrap_or(1)
    }
    fn fill_interval(&self) -> u64 {
        self.forward_fill_interval.unwrap_or(20)
    }
    fn connection_retake_interval(&self) -> u64 {
        self.forward_connection_retake_interval.unwrap_or(1)
    }
    fn check_interval(&self) -> u64 {
        self.forward_check_interval.unwrap_or(10)
    }
    fn connection_max_alive(&self) -> i64 {
        self.forward_connection_max_alive.unwrap_or(300)
    }
    fn heartbeat_timeout(&self) -> u64 {
        self.forward_heartbeat_timeout.unwrap_or(5)
    }
}
impl ProxyTcpConnectionInfoSelector for ProxyConfig {
    fn select_proxy_tcp_connection_info(&self) -> Result<ProxyTcpConnectionInfo, CommonError> {
        let forward_proxy_infos = self.forward_proxies.as_deref().ok_or(CommonError::Other(
            "Forward proxy information not defined in configuration".to_string(),
        ))?;
        let select_index = random::<u32>() % forward_proxy_infos.len() as u32;
        let forward_proxy_info = &forward_proxy_infos[select_index as usize];
        let proxy_addresses =
            parse_to_socket_addresses(vec![forward_proxy_info.proxy_address.clone()].iter())?;
        Ok(ProxyTcpConnectionInfo::new(
            proxy_addresses,
            forward_proxy_info.proxy_auth.to_owned(),
            self.forward_proxy_framed_buffer_size().unwrap_or(65536),
            self.forward_proxy_connect_timeout().unwrap_or(120),
        ))
    }
}
