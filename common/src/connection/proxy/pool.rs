use crate::crypto::RsaCryptoRepository;
use crate::error::CommonError;
use crate::{ProxyTcpConnection, ProxyTcpConnectionInfo};
use concurrent_queue::{ConcurrentQueue, PopError, PushError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use tokio::time::sleep;
use tracing::{debug, error};

pub trait ProxyTcpConnectionInfoSelector {
    fn select_proxy_tcp_connection_info(&self) -> Result<ProxyTcpConnectionInfo, CommonError>;
}

pub trait ProxyTcpConnectionPoolConfig {
    fn max_pool_size(&self) -> Option<usize>;
    fn fill_interval(&self) -> Option<u64>;

    fn connection_retake_interval(&self) -> Option<u64>;
}
/// The connection pool for proxy connection.
pub struct ProxyTcpConnectionPool<C, S, R>
where
    C: ProxyTcpConnectionPoolConfig + Send + Sync + 'static,
    S: ProxyTcpConnectionInfoSelector + Send + Sync + 'static,
    R: RsaCryptoRepository + Sync + Send + 'static,
{
    /// The pool to store the proxy connection
    pool: Arc<ConcurrentQueue<ProxyTcpConnection>>,
    /// If the filling process is happening
    filling: Arc<AtomicBool>,
    config: Arc<C>,
    rsa_crypto_repo: Arc<R>,
    proxy_tcp_connection_info_selector: Arc<S>,
}
impl<C, S, R> ProxyTcpConnectionPool<C, S, R>
where
    C: ProxyTcpConnectionPoolConfig + Send + Sync + 'static,
    S: ProxyTcpConnectionInfoSelector + Send + Sync + 'static,
    R: RsaCryptoRepository + Sync + Send + 'static,
{
    /// Create the proxy connection pool
    pub async fn new(
        config: Arc<C>,
        rsa_crypto_repo: Arc<R>,
        proxy_tcp_connection_info_selector: Arc<S>,
    ) -> Result<Self, CommonError> {
        let pool = Arc::new(ConcurrentQueue::bounded(config.max_pool_size().ok_or(
            CommonError::Other(
                "Proxy connection max pool size not defined in configuration".to_string(),
            ),
        )?));
        let filling = Arc::new(AtomicBool::new(false));
        match &config.fill_interval() {
            None => {
                Self::fill_pool(
                    pool.clone(),
                    config.clone(),
                    rsa_crypto_repo.clone(),
                    proxy_tcp_connection_info_selector.clone(),
                    filling.clone(),
                )
                .await;
            }
            Some(interval) => {
                let config = config.clone();
                let interval = *interval;
                let pool = pool.clone();
                let filling = filling.clone();
                let rsa_crypto_repo = rsa_crypto_repo.clone();
                let proxy_tcp_connection_info_selector = proxy_tcp_connection_info_selector.clone();
                tokio::spawn(async move {
                    loop {
                        debug!("Starting connection pool auto filling loop.");
                        Self::fill_pool(
                            pool.clone(),
                            config.clone(),
                            rsa_crypto_repo.clone(),
                            proxy_tcp_connection_info_selector.clone(),
                            filling.clone(),
                        )
                        .await;
                        sleep(Duration::from_secs(interval)).await;
                    }
                });
            }
        }

        Ok(Self {
            pool,
            config,
            rsa_crypto_repo,
            filling,
            proxy_tcp_connection_info_selector,
        })
    }

    pub async fn take_proxy_connection(&self) -> Result<ProxyTcpConnection, CommonError> {
        Self::concrete_take_proxy_connection(
            self.pool.clone(),
            self.config.clone(),
            self.rsa_crypto_repo.clone(),
            self.proxy_tcp_connection_info_selector.clone(),
            self.filling.clone(),
        )
        .await
    }

    /// The concrete take proxy connection implementation
    async fn concrete_take_proxy_connection(
        pool: Arc<ConcurrentQueue<ProxyTcpConnection>>,
        config: Arc<C>,
        rsa_crypto_repo: Arc<R>,
        proxy_tcp_connection_info_selector: Arc<S>,
        filling: Arc<AtomicBool>,
    ) -> Result<ProxyTcpConnection, CommonError> {
        loop {
            let pool = pool.clone();
            let current_pool_size = pool.len();
            debug!("Taking proxy connection, current pool size: {current_pool_size}");
            let proxy_tcp_connection = pool.pop();
            match proxy_tcp_connection {
                Err(PopError::Closed) => {
                    return Err(CommonError::Other(
                        "Proxy tcp connection pool closed.".to_string(),
                    ));
                }
                Err(PopError::Empty) => {
                    debug!("No proxy connection available, current pool size: {current_pool_size}");
                    Self::fill_pool(
                        pool,
                        config.clone(),
                        rsa_crypto_repo.clone(),
                        proxy_tcp_connection_info_selector.clone(),
                        filling.clone(),
                    )
                    .await;
                    continue;
                }
                Ok(proxy_tcp_connection) => {
                    debug!("Proxy connection available, current pool size before take: {current_pool_size}");
                    return Ok(proxy_tcp_connection);
                }
            }
        }
    }

    /// Fill the pool with proxy connection
    async fn fill_pool(
        pool: Arc<ConcurrentQueue<ProxyTcpConnection>>,
        config: Arc<C>,
        rsa_crypto_repo: Arc<R>,
        proxy_tcp_connection_info_selector: Arc<S>,
        filling: Arc<AtomicBool>,
    ) {
        let max_pool_size = match config.max_pool_size() {
            None => {
                return;
            }
            Some(max_pool_size) => max_pool_size,
        };
        if pool.len() == max_pool_size {
            debug!("Cancel filling proxy connection pool, no need to start filling task(outside task).");
            return;
        }

        tokio::spawn(async move {
            if filling.load(Ordering::Relaxed) {
                debug!(
                    "Cancel filling proxy connection pool, because of filling process is running."
                );
                return;
            }
            if pool.len() == max_pool_size {
                debug!(
                    "Cancel filling proxy connection pool, no need to start filling task(inside task)."
                );
                return;
            }
            debug!("Begin to fill proxy connection pool");
            filling.store(true, Ordering::Relaxed);
            let (proxy_tcp_connection_tx, mut proxy_tcp_connection_rx) =
                channel::<ProxyTcpConnection>(max_pool_size);
            let current_pool_size = pool.len();
            debug!("Current pool size: {current_pool_size}");
            for _ in current_pool_size..max_pool_size {
                let rsa_crypto_repo = rsa_crypto_repo.clone();
                let proxy_tcp_connection_tx = proxy_tcp_connection_tx.clone();
                let proxy_tcp_connection_info_selector = proxy_tcp_connection_info_selector.clone();
                tokio::spawn(async move {
                    let proxy_addresses = match proxy_tcp_connection_info_selector
                        .select_proxy_tcp_connection_info()
                    {
                        Ok(info) => info,
                        Err(e) => {
                            error!("Fail to create proxy tcp connection because of error happen on select proxy address: {e:?}");
                            return;
                        }
                    };
                    match ProxyTcpConnection::create(proxy_addresses, rsa_crypto_repo.as_ref())
                        .await
                    {
                        Ok(proxy_tcp_connection) => {
                            if let Err(e) = proxy_tcp_connection_tx.send(proxy_tcp_connection).await
                            {
                                error!("Fail to send proxy tcp connection: {e:?}")
                            }
                        }
                        Err(e) => {
                            error!("Failed to create proxy connection: {e}");
                        }
                    }
                });
            }
            drop(proxy_tcp_connection_tx);
            debug!("Waiting for proxy connection creation");
            while let Some(proxy_tcp_connection) = proxy_tcp_connection_rx.recv().await {
                match pool.push(proxy_tcp_connection) {
                    Ok(()) => {
                        debug!(
                            "Proxy connection creation add to pool, current pool size: {}",
                            pool.len()
                        );
                    }
                    Err(PushError::Full(proxy_tcp_connection)) => {
                        error!("Failed to push connection into pool because of pool full: {proxy_tcp_connection:?}");
                    }
                    Err(PushError::Closed(proxy_tcp_connection)) => {
                        error!("Failed to push connection into pool because of pool closed: {proxy_tcp_connection:?}");
                    }
                }
            }
            filling.store(false, Ordering::Relaxed);
        });
    }
}
