use crate::config::ServerConfig;
use crate::error::CommonError;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info};

pub struct ServerState {
    values: HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>,
}
impl ServerState {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
    pub fn add_value<T>(&mut self, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.values.insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn get_value<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        let val = self.values.get(&TypeId::of::<T>())?;
        val.downcast_ref::<T>()
    }
}

#[async_trait::async_trait]
pub trait Server<C>
where
    C: ServerConfig + Send + Sync + 'static,
{
    fn new(config: Arc<C>, server_state: ServerState) -> Self;

    fn config(&self) -> Arc<C>;

    fn server_state(&self) -> Arc<ServerState>;

    async fn run<F, Fut>(&self, connection_handler: F) -> Result<(), CommonError>
    where
        F: Fn(Arc<C>, Arc<ServerState>, TcpStream, SocketAddr) -> Fut
            + Send
            + Sync
            + Clone
            + 'static,
        Fut: Future<Output = Result<(), CommonError>> + Send + 'static,
    {
        let config = self.config();
        let server_state = self.server_state();
        let listener = if config.ip_v6() {
            debug!(
                "Starting server listener with IPv6 on port: {}",
                config.server_port()
            );
            TcpListener::bind(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                config.server_port(),
            ))
            .await?
        } else {
            debug!(
                "Starting server listener with IPv4 on port: {}",
                config.server_port()
            );
            TcpListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                config.server_port(),
            ))
            .await?
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
            agent_tcp_stream.set_nodelay(true)?;
            agent_tcp_stream.set_linger(None)?;
            let config = config.clone();
            let server_state = server_state.clone();
            let connection_handler = connection_handler.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    connection_handler(config, server_state, agent_tcp_stream, agent_socket_address)
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
}

pub struct CommonServer<C>
where
    C: ServerConfig + Send + Sync + 'static,
{
    config: Arc<C>,
    server_state: Arc<ServerState>,
}

#[async_trait::async_trait]
impl<C> Server<C> for CommonServer<C>
where
    C: ServerConfig + Send + Sync + 'static,
{
    fn new(config: Arc<C>, server_state: ServerState) -> Self {
        Self {
            config,
            server_state: Arc::new(server_state),
        }
    }
    fn config(&self) -> Arc<C> {
        self.config.clone()
    }
    fn server_state(&self) -> Arc<ServerState> {
        self.server_state.clone()
    }
}
