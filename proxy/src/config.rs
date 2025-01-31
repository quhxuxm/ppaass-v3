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
#[derive(Serialize, Deserialize, Accessors)]
pub struct ProxyConfig {
    #[access(get)]
    ip_v6: bool,
    #[access(get)]
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
    #[access(get)]
    forward_proxies: Option<Vec<ForwardProxyInfo>>,
    #[access(get)]
    forward_rsa_dir: Option<PathBuf>,

    max_pool_size: Option<usize>,
    fill_interval: Option<u64>,
    connection_retake_interval: Option<u64>,
    #[access(get(cp))]
    proxy_to_destination_data_relay_buffer_size: usize,
    #[access(get(cp))]
    destination_to_proxy_data_relay_buffer_size: usize,
}

#[derive(Serialize, Deserialize, Accessors)]
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
    fn max_pool_size(&self) -> Option<usize> {
        self.max_pool_size
    }
    fn fill_interval(&self) -> Option<u64> {
        self.fill_interval
    }
    fn connection_retake_interval(&self) -> Option<u64> {
        self.connection_retake_interval
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
        ))
    }
}
