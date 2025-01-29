use crate::UnifiedAddress;
use serde::{Deserialize, Serialize};
/// The encryption in Handshake message used to
/// switch the encryption key
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Encryption {
    /// The data will send in plain
    Plain,
    /// The data will send with aes encryption
    Aes(#[serde(with = "crate::hex")] Vec<u8>),
    /// The data will send with blowfish encryption
    Blowfish(#[serde(with = "crate::hex")] Vec<u8>),
}

/// The handshake message between agent and proxy.
/// When the tcp connection created between agent and proxy,
/// the handshake will happen as the first message used to
/// communicate the authentication information and exchange
/// the encryption key.
///
/// The **encryption key** is encrypted with **RSA private key**
/// which assigned to each user. The other side should decrypt
/// the encryption key with **RSA public key** to raw key.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HandshakeRequest {
    /// The authentication information, usually it should be a JWT or
    /// a username, or even username&password with some kind of format
    pub authentication: String,
    /// The encryption used to carry the **encryption key**
    pub encryption: Encryption,
}

/// The handshake response, exchange the proxy side encryption
/// to agent
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HandshakeResponse {
    /// The encryption used to carry the **encryption key**
    pub encryption: Encryption,
}

/// The tcp destination initialize message used to initialize the destination
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TunnelInitRequest {
    /// The tcp destination
    Tcp {
        /// The destination that the destination is going to connect
        destination_address: UnifiedAddress,
        /// If the destination should keep alive
        keep_alive: bool,
    },
    /// The udp destination
    Udp,
}

/// The failure reason for destination init
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TunnelInitFailureReason {
    /// Authenticate the user fail
    AuthenticateFail,
    /// Initialize destination with destination fail
    InitWithDestinationFail,
}

/// The tcp destination initialize message used to initialize the destination
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TunnelInitResponse {
    Success,
    Failure(TunnelInitFailureReason),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UdpRelayDataRequest {
    pub destination_address: UnifiedAddress,
    pub source_address: UnifiedAddress,
    #[serde(with = "crate::hex")]
    pub payload: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UdpRelayDataResponse {
    pub destination_address: UnifiedAddress,
    pub source_address: UnifiedAddress,
    #[serde(with = "crate::hex")]
    pub payload: Vec<u8>,
}
