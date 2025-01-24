use crate::error::ServerError;
use crate::tunnel::Tunnel;
use crate::ServerConfig;
use ppaass_common::crypto::RsaCryptoRepository;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tracing::{debug, error, info};
pub struct Server<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<ServerConfig>,
    rsa_crypto_repo: Arc<T>,
}

impl<T> Server<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    pub fn new(config: Arc<ServerConfig>, rsa_crypto_repo: Arc<T>) -> Self {
        Self {
            config,
            rsa_crypto_repo,
        }
    }

    pub fn run(self) -> Result<(), ServerError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(*self.config.worker_threads())
            .build()?;
        runtime.block_on(async move {
            if let Err(e) = self.running_server().await {
                error!("Fail to start server: {}", e);
            }
        });
        Ok(())
    }

    async fn running_server(self) -> Result<(), ServerError> {
        let listener = if *self.config.ip_v6() {
            debug!(
                "Starting server listener with IPv6 on port: {}",
                self.config.port()
            );
            TcpListener::bind(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                *self.config.port(),
            ))
            .await?
        } else {
            debug!(
                "Starting server listener with IPv4 on port: {}",
                self.config.port()
            );
            TcpListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                *self.config.port(),
            ))
            .await?
        };
        info!("Server listening on port: {}", self.config.port());
        loop {
            let (agent_tcp_stream, agent_socket_address) = listener.accept().await?;
            let config = self.config.clone();
            let rsa_crypto_repo = self.rsa_crypto_repo.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_agent_connection(
                    config,
                    rsa_crypto_repo,
                    agent_tcp_stream,
                    agent_socket_address,
                )
                .await
                {
                    error!(
                        "Fail to handle agent tcp connection [{agent_socket_address}]: {}",
                        e
                    );
                }
            });
        }
    }

    async fn handle_agent_connection(
        config: Arc<ServerConfig>,
        rsa_crypto_repo: Arc<T>,
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
    ) -> Result<(), ServerError> {
        let tunnel = Tunnel::new(
            config,
            agent_tcp_stream,
            agent_socket_address,
            rsa_crypto_repo,
        );
        tunnel.run().await
    }
}
