use crate::error::CommonError;
use crate::user::repo::fs::USER_INFO_ADDITION_INFO_PROXY_SERVERS;
use crate::user::UserInfo;
use crate::{
    parse_to_socket_addresses, ProxyTcpConnection, ProxyTcpConnectionInfo,
    ProxyTcpConnectionTunnelCtlState,
};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::fmt::Debug;

use crate::config::{ProxyTcpConnectionConfig, ProxyTcpConnectionPoolConfig, UserInfoConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error};
pub trait ProxyTcpConnectionInfoSelector {
    fn select_proxy_tcp_connection_info(
        &self,
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
}

#[derive(Debug)]
struct ProxyTcpConnectionPoolElement<C, T>
where
    C: ProxyTcpConnectionPoolConfig + UserInfoConfig + Debug + Send + Sync + 'static,
    T: ProxyTcpConnectionConfig + UserInfoConfig + Debug + Send + Sync + 'static,
{
    proxy_tcp_connection: ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState>,
    create_time: DateTime<Utc>,
    last_check_time: DateTime<Utc>,
    last_check_duration: i64,
    pool_config: Arc<C>,
    connection_config: Arc<T>,
}
impl<C, T> ProxyTcpConnectionPoolElement<C, T>
where
    C: ProxyTcpConnectionPoolConfig + UserInfoConfig + Debug + Send + Sync + 'static,
    T: ProxyTcpConnectionConfig + UserInfoConfig + Debug + Send + Sync + 'static,
{
    pub fn new(
        proxy_tcp_connection: ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState>,
        pool_config: Arc<C>,
        connection_config: Arc<T>,
    ) -> Self {
        Self {
            proxy_tcp_connection,
            last_check_time: Utc::now(),
            create_time: Utc::now(),
            last_check_duration: 0,
            pool_config,
            connection_config,
        }
    }
    pub fn need_check(&self) -> bool {
        let now = Utc::now();
        let delta = now - self.last_check_time;
        delta.num_seconds() > self.pool_config.check_interval() as i64
    }
    pub fn need_close(&self) -> bool {
        let now = Utc::now();
        let delta = now - self.create_time;
        delta.num_seconds() > self.pool_config.connection_max_alive()
    }
}
/// The connection pool for proxy connection.
pub struct ProxyTcpConnectionPool<C, T, S>
where
    C: ProxyTcpConnectionPoolConfig + UserInfoConfig + Debug + Send + Sync + 'static,
    T: ProxyTcpConnectionConfig + UserInfoConfig + Debug + Send + Sync + 'static,
    S: ProxyTcpConnectionInfoSelector + Send + Sync + 'static,
{
    /// The pool to store the proxy connection
    pool: Arc<Mutex<Vec<ProxyTcpConnectionPoolElement<C, T>>>>,
    pool_config: Arc<C>,
    connection_config: Arc<T>,
    user_info: Arc<UserInfo>,
    proxy_tcp_connection_info_selector: Arc<S>,
}
impl<C, T, S> ProxyTcpConnectionPool<C, T, S>
where
    C: ProxyTcpConnectionPoolConfig
        + ProxyTcpConnectionConfig
        + UserInfoConfig
        + Debug
        + Send
        + Sync
        + 'static,
    T: ProxyTcpConnectionConfig + UserInfoConfig + Debug + Send + Sync + 'static,
    S: ProxyTcpConnectionInfoSelector + Send + Sync + 'static,
{
    /// Create the proxy connection pool
    pub async fn new(
        pool_config: Arc<C>,
        connection_config: Arc<T>,
        user_info: Arc<UserInfo>,
        proxy_tcp_connection_info_selector: Arc<S>,
    ) -> Result<Self, CommonError> {
        let pool = Arc::new(Mutex::new(Vec::new()));
        let interval = pool_config.fill_interval();
        let pool_clone = pool.clone();
        let user_info_clone = user_info.clone();
        let proxy_tcp_connection_info_selector_clone = proxy_tcp_connection_info_selector.clone();
        let pool_config_clone = pool_config.clone();
        tokio::spawn(async move {
            loop {
                debug!("Starting connection pool auto filling loop.");
                Self::fill_pool(
                    pool_clone.clone(),
                    pool_config_clone.clone(),
                    user_info_clone.clone(),
                    proxy_tcp_connection_info_selector_clone.clone(),
                )
                .await;
                sleep(Duration::from_secs(interval)).await;
            }
        });
        Self::start_connection_check_task(pool_config_clone.clone(), pool.clone());
        Ok(Self {
            pool,
            pool_config,
            connection_config,
            user_info,
            proxy_tcp_connection_info_selector,
        })
    }
    pub async fn take_proxy_connection(
        &self,
    ) -> Result<ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState>, CommonError> {
        Self::concrete_take_proxy_connection(
            self.pool.clone(),
            self.config.clone(),
            self.user_info.clone(),
            self.proxy_tcp_connection_info_selector.clone(),
        )
        .await
    }

    async fn check_proxy_connection(
        proxy_tcp_connection_pool_element: &mut ProxyTcpConnectionPoolElement<C>,
        config: &C,
    ) -> Result<(), CommonError> {
        debug!("Checking proxy connection : {proxy_tcp_connection_pool_element:?}");
        let check_duration = proxy_tcp_connection_pool_element
            .proxy_tcp_connection
            .heartbeat(config.heartbeat_timeout())
            .await?;
        proxy_tcp_connection_pool_element.last_check_duration = check_duration;
        Ok(())
    }

    fn start_connection_check_task(
        pool_config: Arc<C>,
        connection_config: Arc<T>,
        pool: Arc<Mutex<Vec<ProxyTcpConnectionPoolElement<C, T>>>>,
    ) {
        tokio::spawn(async move {
            loop {
                let mut pool_lock = pool.lock().await;
                debug!(
                    "Start checking connection pool loop, current pool size: {} ",
                    pool_lock.len()
                );
                let (checking_tx, mut checking_rx) =
                    channel::<ProxyTcpConnectionPoolElement<C, T>>(pool_config.max_pool_size());
                'for_each_connection: loop {
                    let mut proxy_tcp_connection_pool_element = match pool_lock.pop() {
                        None => break 'for_each_connection,
                        Some(proxy_tcp_connection_pool_element) => {
                            proxy_tcp_connection_pool_element
                        }
                    };
                    if !proxy_tcp_connection_pool_element.need_check() {
                        if let Err(e) = checking_tx.send(proxy_tcp_connection_pool_element).await {
                            error!("Fail to push proxy connection back to pool: {}", e);
                        }
                        continue 'for_each_connection;
                    }
                    if proxy_tcp_connection_pool_element.need_close() {
                        debug!("Close proxy connection because of it exceed max life time: {proxy_tcp_connection_pool_element:?}");
                        continue 'for_each_connection;
                    }
                    let checking_tx = checking_tx.clone();
                    let pool_config = pool_config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::check_proxy_connection(
                            &mut proxy_tcp_connection_pool_element,
                            &pool_config,
                        )
                        .await
                        {
                            error!("Failed to check proxy connection: {}", e);
                            return;
                        };
                        if let Err(e) = checking_tx.send(proxy_tcp_connection_pool_element).await {
                            error!("Fail to push proxy connection back to pool: {}", e);
                        };
                    });
                }
                drop(checking_tx);
                while let Some(mut proxy_connection) = checking_rx.recv().await {
                    if pool_lock.len() >= pool_config.max_pool_size() {
                        tokio::spawn(async move {
                            if let Err(e) = proxy_connection.proxy_tcp_connection.close().await {
                                error!("Failed to close proxy connection: {}", e);
                            };
                        });
                        continue;
                    }
                    pool_lock.push(proxy_connection);
                }
                pool_lock.sort_by(|a, b| {
                    let comp = a.last_check_duration.cmp(&b.last_check_duration);
                    if Ordering::Equal == comp {
                        a.last_check_time.cmp(&b.last_check_time)
                    } else {
                        comp
                    }
                });
                drop(pool_lock);
                sleep(Duration::from_secs(pool_config.check_interval())).await;
            }
        });
    }
    /// The concrete take proxy connection implementation
    async fn concrete_take_proxy_connection(
        pool: Arc<Mutex<Vec<ProxyTcpConnectionPoolElement<C, T>>>>,
        pool_config: Arc<C>,
        connection_config: Arc<T>,
        user_info: Arc<UserInfo>,
        proxy_tcp_connection_info_selector: Arc<S>,
    ) -> Result<ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState>, CommonError> {
        let mut pool_lock = pool.lock().await;
        debug!(
            "Taking proxy connection, current pool size: {}",
            pool_lock.len()
        );
        let proxy_tcp_connection_element = pool_lock.pop();
        match proxy_tcp_connection_element {
            None => {
                drop(pool_lock);
                Self::fill_pool(
                    pool.clone(),
                    pool_config.clone(),
                    user_info.clone(),
                    proxy_tcp_connection_info_selector.clone(),
                )
                .await;
                Box::pin(Self::concrete_take_proxy_connection(
                    pool,
                    pool_config,
                    connection_config,
                    user_info,
                    proxy_tcp_connection_info_selector,
                ))
                .await
            }
            Some(proxy_tcp_connection_element) => {
                debug!(
                    "Proxy connection available, current pool size before take: {}",
                    pool_lock.len()
                );
                Ok(proxy_tcp_connection_element.proxy_tcp_connection)
            }
        }
    }
    /// Fill the pool with proxy connection
    async fn fill_pool(
        pool: Arc<Mutex<Vec<ProxyTcpConnectionPoolElement<C, T>>>>,
        pool_config: Arc<C>,
        connection_config: Arc<T>,
        user_info: Arc<UserInfo>,
        proxy_tcp_connection_info_selector: Arc<S>,
    ) {
        let max_pool_size = pool_config.max_pool_size();
        let mut pool_lock = pool.lock().await;
        if pool_lock.len() >= max_pool_size {
            debug!("Cancel filling proxy connection pool, because the pool size exceed max, current pool size: {}, max pool size: {}", pool_lock.len(), max_pool_size);
            return;
        }
        debug!(
            "Begin to fill proxy connection pool, current pool size:{}",
            pool_lock.len()
        );
        let (proxy_tcp_connection_tx, mut proxy_tcp_connection_rx) =
            channel::<ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState>>(max_pool_size);

        for _ in pool_lock.len()..max_pool_size {
            let proxy_tcp_connection_tx = proxy_tcp_connection_tx.clone();
            let proxy_tcp_connection_info_selector = proxy_tcp_connection_info_selector.clone();
            let user_info = user_info.clone();
            let pool_config = pool_config.clone();
            tokio::spawn(async move {
                let proxy_addresses = match proxy_tcp_connection_info_selector
                    .select_proxy_tcp_connection_info(pool_config.username(), user_info.as_ref())
                {
                    Ok(info) => info,
                    Err(e) => {
                        error!("Fail to create proxy tcp connection because of error happen on select proxy address: {e:?}");
                        return;
                    }
                };
                debug!("Going to create proxy tcp connection with forward proxy address: {proxy_addresses:?}");
                match ProxyTcpConnection::create(
                    proxy_addresses.clone(),
                    user_info.as_ref(),
                    config.proxy_frame_size(),
                    config.proxy_connect_timeout(),
                )
                .await
                {
                    Ok(proxy_tcp_connection) => {
                        debug!("Create forward proxy tcp connection success: {proxy_addresses:?}");
                        if let Err(e) = proxy_tcp_connection_tx.send(proxy_tcp_connection).await {
                            error!("Fail to send proxy tcp connection: {e:?}")
                        }
                    }
                    Err(e) => {
                        error!("Failed to create proxy connection [{proxy_addresses:?}]: {e}");
                    }
                }
            });
        }

        drop(proxy_tcp_connection_tx);
        debug!("Waiting for proxy connection creation result.");
        while let Some(proxy_tcp_connection) = proxy_tcp_connection_rx.recv().await {
            let proxy_tcp_connection_element_to_push =
                ProxyTcpConnectionPoolElement::new(proxy_tcp_connection, config.clone());
            pool_lock.push(proxy_tcp_connection_element_to_push)
        }
    }
}
