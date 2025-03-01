mod connection;
mod pool;
use crate::error::CommonError;
use crate::parse_to_socket_addresses;
use crate::user::UserInfo;
use crate::user::repo::fs::USER_INFO_ADDITION_INFO_PROXY_SERVERS;
pub use connection::*;
pub use pool::*;
fn select_proxy_tcp_connection_info(
    username: &str,
    user_info: &UserInfo,
) -> Result<ProxyTcpConnectionInfo, CommonError> {
    let proxy_addresses = user_info
        .get_additional_info::<Vec<String>>(USER_INFO_ADDITION_INFO_PROXY_SERVERS)
        .ok_or(CommonError::Other(format!(
            "No proxy servers defined in user info configuration: {user_info:?}"
        )))?;
    let proxy_addresses = parse_to_socket_addresses(proxy_addresses.iter())?;

    let select_index = rand::random::<u64>() % proxy_addresses.len() as u64;
    let proxy_address = proxy_addresses[select_index as usize];

    Ok(ProxyTcpConnectionInfo::new(
        proxy_address,
        username.to_owned(),
    ))
}
