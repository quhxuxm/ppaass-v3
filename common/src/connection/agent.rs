use crate::connection::codec::{
    HandshakeRequestDecoder, HandshakeResponseEncoder, TunnelControlRequestResponseCodec,
};
use crate::connection::CryptoLengthDelimitedFramed;
use crate::error::CommonError;
use crate::user::repo::fs::USER_INFO_ADDITION_INFO_EXPIRED_DATE_TIME;
use crate::user::UserInfoRepository;
use crate::{
    random_generate_encryption, rsa_decrypt_encryption, rsa_encrypt_encryption, FramedConnection,
};
use chrono::{DateTime, Utc};
use futures_util::SinkExt;
use futures_util::StreamExt;
use ppaass_protocol::{
    Encryption, HandshakeRequest, HandshakeResponse, HeartbeatResponse, TunnelControlRequest,
    TunnelControlResponse, TunnelInitRequest, TunnelInitResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, FramedParts};
use tokio_util::io::{SinkWriter, StreamReader};
use tracing::debug;
pub struct AgentTcpConnectionNewState {}
pub struct AgentTcpConnectionTunnelCtlState {
    tunnel_ctl_request_response_framed: Framed<TcpStream, TunnelControlRequestResponseCodec>,
    proxy_encryption: Arc<Encryption>,
    agent_encryption: Arc<Encryption>,
}

impl FramedConnection<AgentTcpConnectionNewState> {
    pub async fn create<R>(
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        user_info_repo: &R,
        frame_buffer_size: usize,
    ) -> Result<FramedConnection<AgentTcpConnectionTunnelCtlState>, CommonError>
    where
        R: UserInfoRepository + Sync + Send + 'static,
    {
        let mut handshake_request_framed =
            Framed::new(agent_tcp_stream, HandshakeRequestDecoder::new());
        let HandshakeRequest {
            authentication,
            encryption,
        } = handshake_request_framed
            .next()
            .await
            .ok_or(CommonError::ConnectionExhausted(agent_socket_address))??;
        let user_info = user_info_repo
            .get_user(&authentication)
            .await?
            .ok_or(CommonError::RsaCryptoNotFound(authentication.clone()))?;
        let user_info = user_info.read().await;
        let user_expired_time = user_info
            .get_additional_info::<DateTime<Utc>>(USER_INFO_ADDITION_INFO_EXPIRED_DATE_TIME);
        if let Some(user_expired_time) = user_expired_time {
            if Utc::now() > *user_expired_time {
                return Err(CommonError::UserExpired(authentication));
            }
        }
        let agent_encryption =
            rsa_decrypt_encryption(&encryption, user_info.rsa_crypto())?.into_owned();
        let proxy_encryption = random_generate_encryption();
        let encrypted_proxy_encryption =
            rsa_encrypt_encryption(&proxy_encryption, user_info.rsa_crypto())?;
        let handshake_response = HandshakeResponse {
            encryption: encrypted_proxy_encryption.into_owned(),
        };
        let FramedParts {
            io: agent_tcp_stream,
            ..
        } = handshake_request_framed.into_parts();
        let mut handshake_response_framed =
            Framed::new(agent_tcp_stream, HandshakeResponseEncoder::new());
        handshake_response_framed.send(handshake_response).await?;
        let FramedParts {
            io: agent_tcp_stream,
            ..
        } = handshake_response_framed.into_parts();
        let proxy_encryption = Arc::new(proxy_encryption);
        let agent_encryption = Arc::new(agent_encryption);
        Ok(FramedConnection {
            socket_address: agent_socket_address,

            frame_buffer_size,
            state: AgentTcpConnectionTunnelCtlState {
                proxy_encryption: proxy_encryption.clone(),
                agent_encryption: agent_encryption.clone(),
                tunnel_ctl_request_response_framed: Framed::with_capacity(
                    agent_tcp_stream,
                    TunnelControlRequestResponseCodec::new(agent_encryption, proxy_encryption),
                    frame_buffer_size,
                ),
            },
        })
    }
}
impl FramedConnection<AgentTcpConnectionTunnelCtlState> {
    pub async fn wait_tunnel_init(&mut self) -> Result<TunnelInitRequest, CommonError> {
        loop {
            let tunnel_ctl_request = self
                .state
                .tunnel_ctl_request_response_framed
                .next()
                .await
                .ok_or(CommonError::ConnectionExhausted(self.socket_address))??;
            match tunnel_ctl_request {
                TunnelControlRequest::Heartbeat(heartbeat_request) => {
                    debug!(
                        "Receive heartbeat request from agent connection [{}]: {heartbeat_request:?}",
                        self.socket_address
                    );
                    let heartbeat_response =
                        TunnelControlResponse::Heartbeat(HeartbeatResponse::new());
                    self.state
                        .tunnel_ctl_request_response_framed
                        .send(heartbeat_response)
                        .await?;
                    continue;
                }
                TunnelControlRequest::TunnelInit(tunnel_init_request) => {
                    return Ok(tunnel_init_request);
                }
            }
        }
    }

    pub async fn response_tcp_tunnel_init(
        mut self,
        tunnel_init_response: TunnelInitResponse,
    ) -> Result<
        FramedConnection<
            SinkWriter<StreamReader<CryptoLengthDelimitedFramed<TcpStream>, BytesMut>>,
        >,
        CommonError,
    > {
        let tunnel_ctl_response = TunnelControlResponse::TunnelInit(tunnel_init_response);
        self.state
            .tunnel_ctl_request_response_framed
            .send(tunnel_ctl_response)
            .await?;
        let FramedParts { io, .. } = self.state.tunnel_ctl_request_response_framed.into_parts();
        Ok(FramedConnection {
            socket_address: self.socket_address,
            state: SinkWriter::new(StreamReader::new(CryptoLengthDelimitedFramed::new(
                io,
                self.state.agent_encryption,
                self.state.proxy_encryption,
                self.frame_buffer_size,
            ))),
            frame_buffer_size: self.frame_buffer_size,
        })
    }
}
