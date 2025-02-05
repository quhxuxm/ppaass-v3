use accessory::Accessors;
use ppaass_common::config::{ConnectionPoolConfig, ServerConfig};
use ppaass_common::error::CommonError;
use ppaass_common::{
    parse_to_socket_addresses, ProxyTcpConnectionInfo, ProxyTcpConnectionInfoSelector,
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
    agent_frame_buffer_size: usize,
    #[access(get(cp))]
    proxy_to_destination_data_relay_buffer_size: usize,
    #[access(get(cp))]
    destination_to_proxy_data_relay_buffer_size: usize,
    #[access(get)]
    forward: Option<ForwardConfig>,
}

#[derive(Serialize, Deserialize, Accessors, Debug, Clone)]
pub struct ForwardConfig {
    #[access(get(cp))]
    proxy_connect_timeout: u64,
    #[access(get)]
    proxies: Vec<ForwardProxyInfo>,
    #[access(get)]
    rsa_dir: PathBuf,
    #[access(get(cp))]
    proxy_frame_buffer_size: usize,
    #[access(get)]
    connection_pool: Option<ConnectionPoolConfig>,
}

#[derive(Serialize, Deserialize, Accessors, Debug, Clone)]
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

impl ProxyTcpConnectionInfoSelector for ForwardConfig {
    fn select_proxy_tcp_connection_info(&self) -> Result<ProxyTcpConnectionInfo, CommonError> {
        let select_index = random::<u32>() % self.proxies.len() as u32;
        let forward_proxy_info = &self.proxies[select_index as usize];
        let proxy_addresses =
            parse_to_socket_addresses(vec![forward_proxy_info.proxy_address.clone()].iter())?;
        Ok(ProxyTcpConnectionInfo::new(
            proxy_addresses,
            forward_proxy_info.proxy_auth.to_owned(),
            self.proxy_frame_buffer_size(),
            self.proxy_connect_timeout(),
        ))
    }
}
