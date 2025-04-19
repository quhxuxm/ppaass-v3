use crate::ProtocolError;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::{SocketAddr, ToSocketAddrs};
const HTTP_PORT: u16 = 80;

/// The unified address which can support both
/// IP V4, IP V6 and Domain
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum UnifiedAddress {
    Domain { host: String, port: u16 },
    SocketAddress(SocketAddr),
}
impl Display for UnifiedAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedAddress::Domain { host, port } => write!(f, "{}:{}", host, port),
            UnifiedAddress::SocketAddress(socket_addr) => match socket_addr {
                SocketAddr::V4(ip_v4_addr) => {
                    write!(f, "{}:{}", ip_v4_addr.ip(), socket_addr.port())
                }
                SocketAddr::V6(ip_v6_addr) => {
                    write!(f, "{}:{}", ip_v6_addr.ip(), socket_addr.port())
                }
            },
        }
    }
}

impl TryFrom<&str> for UnifiedAddress {
    type Error = ProtocolError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<String> for UnifiedAddress {
    type Error = ProtocolError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Ok(ip_address) = value.parse::<SocketAddr>() {
            Ok(Self::SocketAddress(ip_address))
        } else {
            let domain_parts = value.split(":").collect::<Vec<&str>>();
            match domain_parts.len() {
                parts_num if parts_num > 2 => {
                    Err(ProtocolError::ParseUnifiedAddressToDomainAddress(value))
                }
                2 => {
                    let domain = domain_parts[0];
                    let port = domain_parts[1].parse::<u16>().map_err(|_| {
                        ProtocolError::ParseUnifiedAddressToDomainAddress(value.clone())
                    })?;
                    Ok(Self::Domain {
                        host: domain.to_string(),
                        port,
                    })
                }
                _ => {
                    let domain = domain_parts[0];
                    Ok(Self::Domain {
                        host: domain.to_string(),
                        port: HTTP_PORT,
                    })
                }
            }
        }
    }
}
impl TryFrom<UnifiedAddress> for Vec<SocketAddr> {
    type Error = ProtocolError;
    fn try_from(value: UnifiedAddress) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&UnifiedAddress> for Vec<SocketAddr> {
    type Error = ProtocolError;
    fn try_from(value: &UnifiedAddress) -> Result<Self, Self::Error> {
        match value {
            UnifiedAddress::Domain { host, port } => {
                let socket_addresses = format!("{host}:{port}").to_socket_addrs()?;
                let socket_addresses = socket_addresses.collect::<Vec<SocketAddr>>();
                Ok(socket_addresses)
            }
            UnifiedAddress::SocketAddress(socket_addr) => Ok(vec![*socket_addr]),
        }
    }
}
impl From<SocketAddr> for UnifiedAddress {
    fn from(value: SocketAddr) -> Self {
        UnifiedAddress::SocketAddress(value)
    }
}

impl From<&SocketAddr> for UnifiedAddress {
    fn from(value: &SocketAddr) -> Self {
        UnifiedAddress::SocketAddress(*value)
    }
}
