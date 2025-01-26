use crate::config::ServerConfig;
use crate::crypto::RsaCryptoRepository;
use crate::error::CommonError;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tracing::{debug, error, info};
pub trait Server<C, R>
where
    C: ServerConfig + Send + Sync + 'static,
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    fn new(config: Arc<C>, rsa_crypto_repo: Arc<R>) -> Self;

    fn config(&self) -> &C;

    fn clone_config(&self) -> Arc<C>;

    fn clone_rsa_crypto_repository(&self) -> Arc<R>;

    fn run<F, Fut>(&self, connection_handler: F) -> Result<(), CommonError>
    where
        F: Fn(Arc<C>, Arc<R>, TcpStream, SocketAddr) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<(), CommonError>> + Send + Sync + 'static,
    {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(self.config().worker_thread_number())
            .build()?;
        let config = self.clone_config();
        let rsa_crypto_repo = self.clone_rsa_crypto_repository();
        runtime.block_on(async move {
            let listener = if config.ip_v6() {
                debug!(
                    "Starting server listener with IPv6 on port: {}",
                    config.server_port()
                );
                match TcpListener::bind(SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                    config.server_port(),
                ))
                .await
                {
                    Ok(listener) => listener,
                    Err(e) => {
                        error!("Failed to start server listener with IPv6 on port: {}", e);
                        return;
                    }
                }
            } else {
                debug!(
                    "Starting server listener with IPv4 on port: {}",
                    config.server_port()
                );
                match TcpListener::bind(SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    config.server_port(),
                ))
                .await
                {
                    Ok(listener) => listener,
                    Err(e) => {
                        error!("Failed to start server listener with IPv4 on port: {}", e);
                        return;
                    }
                }
            };
            info!("Server listening on port: {}", config.server_port());
            loop {
                let (agent_tcp_stream, agent_socket_address) = match listener.accept().await {
                    Ok(agent_tcp_accept_result) => agent_tcp_accept_result,
                    Err(e) => {
                        error!("Failed to accept connection with IPv4 on port: {}", e);
                        continue;
                    }
                };
                let config = config.clone();
                let rsa_crypto_repo = rsa_crypto_repo.clone();
                let connection_handler = connection_handler.clone();
                tokio::spawn(async move {
                    if let Err(e) = connection_handler(
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
        });
        Ok(())
    }
}

pub struct CommonServer<C, R>
where
    C: ServerConfig + Send + Sync + 'static,
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<C>,
    rsa_crypto_repo: Arc<R>,
}
impl<C, R> Server<C, R> for CommonServer<C, R>
where
    C: ServerConfig + Send + Sync + 'static,
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    fn new(config: Arc<C>, rsa_crypto_repo: Arc<R>) -> Self {
        Self {
            config,
            rsa_crypto_repo,
        }
    }
    fn config(&self) -> &C {
        self.config.as_ref()
    }
    fn clone_config(&self) -> Arc<C> {
        self.config.clone()
    }
    fn clone_rsa_crypto_repository(&self) -> Arc<R> {
        self.rsa_crypto_repo.clone()
    }
}
