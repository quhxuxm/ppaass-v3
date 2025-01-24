use crate::UnifiedAddress;
use serde::{Deserialize, Serialize};
/// The encryption in Handshake message used to
/// switch the encryption key
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Encryption {
    /// The data will send in plain
    Plain,
    /// The data will send with aes encryption
    Aes(Vec<u8>),
    /// The data will send with blowfish encryption
    Blowfish(Vec<u8>),
}

/// The handshake message between agent and server.
/// When the tcp connection created between agent and server,
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

/// The handshake response, exchange the server side encryption
/// to agent
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HandshakeResponse {
    /// The encryption used to carry the **encryption key**
    pub encryption: Encryption,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// The request packet from agent
pub enum AgentRequestPacket {
    /// The destination init request
    Init(TunnelInitRequest),
    /// The relay data request
    Relay(TunnelRelayDataRequest),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// The response packet send to agent
pub enum AgentResponsePacket {
    /// The destination init response
    Init(TunnelInitResponse),
    /// The relay data response
    Relay(TunnelRelayDataResponse),
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
pub enum TunnelRelayDataRequest {
    /// The relay data for tcp destination
    Tcp(Vec<u8>),
    /// The relay data for udp destination
    Udp {
        destination_address: UnifiedAddress,
        source_address: UnifiedAddress,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TunnelRelayDataResponse {
    /// The relay data for tcp destination
    Tcp(Vec<u8>),
    /// The relay data for udp destination
    Udp {
        destination_address: UnifiedAddress,
        source_address: UnifiedAddress,
        payload: Vec<u8>,
    },
}
