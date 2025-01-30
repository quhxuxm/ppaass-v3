use crate::config::ServerConfig;
use crate::crypto::RsaCryptoRepository;
use crate::error::CommonError;
use crate::{ProxyTcpConnectionInfoSelector, ProxyTcpConnectionPool, ProxyTcpConnectionPoolConfig};
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tracing::{debug, error, info};
pub trait Server<C, S, R>
where
    C: ServerConfig + ProxyTcpConnectionPoolConfig + Send + Sync + 'static,
    S: ProxyTcpConnectionInfoSelector + Send + Sync + 'static,
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    fn new(
        config: Arc<C>,
        proxy_tcp_connection_info_selector: Arc<S>,
        rsa_crypto_repo: Arc<R>,
        forward_rsa_crypto_repo: Option<Arc<R>>,
    ) -> Self;

    fn config(&self) -> &C;

    fn clone_config(&self) -> Arc<C>;

    fn clone_rsa_crypto_repository(&self) -> Arc<R>;
    fn clone_forward_rsa_crypto_repository(&self) -> Option<Arc<R>>;
    fn clone_proxy_tcp_connection_info_selector(&self) -> Arc<S>;

    fn run<F, Fut>(&self, connection_handler: F) -> Result<(), CommonError>
    where
        F: Fn(
                Arc<C>,
                Arc<R>,
                TcpStream,
                SocketAddr,
                Option<Arc<ProxyTcpConnectionPool<C, S, R>>>,
            ) -> Fut
            + Send
            + Sync
            + Clone
            + 'static,
        Fut: Future<Output = Result<(), CommonError>> + Send + 'static,
    {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(self.config().worker_thread_number())
            .build()?;
        let config = self.clone_config();
        let proxy_tcp_connection_info_selector = self.clone_proxy_tcp_connection_info_selector();
        let rsa_crypto_repo = self.clone_rsa_crypto_repository();
        let forward_rsa_crypto_repo = self.clone_forward_rsa_crypto_repository();
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

            let proxy_tcp_connection_pool = match config.max_pool_size() {
                None => None,
                Some(_) => {
                    let proxy_pool_rsa_crypto_repo =
                        forward_rsa_crypto_repo.unwrap_or_else(|| rsa_crypto_repo.clone());
                    let proxy_tcp_connection_pool = match ProxyTcpConnectionPool::new(
                        config.clone(),
                        proxy_pool_rsa_crypto_repo,
                        proxy_tcp_connection_info_selector,
                    )
                    .await
                    {
                        Ok(pool) => Arc::new(pool),
                        Err(e) => {
                            error!("Failed to initialize TCP connection pool: {}", e);
                            return;
                        }
                    };
                    Some(proxy_tcp_connection_pool)
                }
            };
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
                let proxy_tcp_connection_pool = proxy_tcp_connection_pool.clone();
                tokio::spawn(async move {
                    if let Err(e) = connection_handler(
                        config,
                        rsa_crypto_repo,
                        agent_tcp_stream,
                        agent_socket_address,
                        proxy_tcp_connection_pool,
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

pub struct CommonServer<C, S, R>
where
    C: ServerConfig + Send + Sync + 'static,
    S: ProxyTcpConnectionInfoSelector + Send + Sync + 'static,
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<C>,
    rsa_crypto_repo: Arc<R>,
    forward_rsa_crypto_repo: Option<Arc<R>>,
    proxy_tcp_connection_info_selector: Arc<S>,
}
impl<C, S, R> Server<C, S, R> for CommonServer<C, S, R>
where
    C: ServerConfig + ProxyTcpConnectionPoolConfig + Send + Sync + 'static,
    S: ProxyTcpConnectionInfoSelector + Send + Sync + 'static,
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    fn new(
        config: Arc<C>,
        proxy_tcp_connection_info_selector: Arc<S>,
        rsa_crypto_repo: Arc<R>,
        forward_rsa_crypto_repo: Option<Arc<R>>,
    ) -> Self {
        Self {
            config,
            rsa_crypto_repo,
            forward_rsa_crypto_repo,
            proxy_tcp_connection_info_selector,
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
    fn clone_forward_rsa_crypto_repository(&self) -> Option<Arc<R>> {
        self.forward_rsa_crypto_repo.clone()
    }
    fn clone_proxy_tcp_connection_info_selector(&self) -> Arc<S> {
        self.proxy_tcp_connection_info_selector.clone()
    }
}
