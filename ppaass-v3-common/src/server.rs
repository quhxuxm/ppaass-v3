use crate::config::RetrieveServerConfig;
use crate::error::CommonError;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};
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

pub enum ServerListener {
    TcpListener(TcpListener),
}

impl ServerListener {
    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr), CommonError> {
        match self {
            ServerListener::TcpListener(tcp_listener) => {
                let accept_result = tcp_listener.accept().await?;
                Ok((accept_result.0, accept_result.1))
            }
        }
    }
}
#[async_trait::async_trait]
pub trait Server<C>
where
    C: RetrieveServerConfig + Send + Sync + 'static,
{
    fn new(config: Arc<C>, server_state: ServerState) -> Self;

    fn config(&self) -> Arc<C>;

    fn server_state(&self) -> Arc<ServerState>;

    async fn run<F1, Fut1, F2, Fut2>(
        &self,
        create_listener: F1,
        connection_handler: F2,
    ) -> Result<(), CommonError>
    where
        F1: Fn(Arc<C>) -> Fut1 + Send + Sync + 'static,
        Fut1: Future<Output = Result<ServerListener, CommonError>> + Send + 'static,
        F2: Fn(Arc<C>, Arc<ServerState>, TcpStream, SocketAddr) -> Fut2
            + Send
            + Sync
            + Clone
            + 'static,
        Fut2: Future<Output = Result<(), CommonError>> + Send + 'static,
    {
        let config = self.config();
        let server_state = self.server_state();
        let listener = create_listener(config.clone()).await?;
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
    C: RetrieveServerConfig + Send + Sync + 'static,
{
    config: Arc<C>,
    server_state: Arc<ServerState>,
}

#[async_trait::async_trait]
impl<C> Server<C> for CommonServer<C>
where
    C: RetrieveServerConfig + Send + Sync + 'static,
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
