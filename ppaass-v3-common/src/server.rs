use crate::config::RetrieveServerConfig;
use crate::error::CommonError;
use crate::event::{DownloadSpeedEvent, LogEvent, LogEventLevel, UploadSpeedEvent};
use crate::publish_server_log_event;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_util::sync::CancellationToken;
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

pub struct ServerGuard {
    pub upload_speed_event_receiver: Receiver<UploadSpeedEvent>,
    pub download_speed_event_receiver: Receiver<DownloadSpeedEvent>,
    pub log_event_receiver: Receiver<LogEvent>,
    pub stop_signal: CancellationToken,
}

pub struct Server<C>
where
    C: RetrieveServerConfig + Send + Sync + 'static,
{
    config: Arc<C>,
    server_state: Arc<ServerState>,
    upload_speed_event_sender: Sender<UploadSpeedEvent>,
    download_speed_event_sender: Sender<DownloadSpeedEvent>,
    log_event_sender: Sender<LogEvent>,
    stop_signal: CancellationToken,
}

impl<C> Server<C>
where
    C: RetrieveServerConfig + Send + Sync + 'static,
{
    pub fn new(config: Arc<C>, server_state: ServerState) -> (Self, ServerGuard) {
        let (upload_speed_event_sender, upload_speed_event_receiver) =
            channel::<UploadSpeedEvent>(1024);
        let (download_speed_event_sender, download_speed_event_receiver) =
            channel::<DownloadSpeedEvent>(1024);
        let (log_event_sender, log_event_receiver) = channel::<LogEvent>(1024);
        let stop_signal = CancellationToken::new();
        (
            Self {
                config,
                server_state: Arc::new(server_state),
                upload_speed_event_sender,
                download_speed_event_sender,
                log_event_sender,
                stop_signal: stop_signal.clone(),
            },
            ServerGuard {
                upload_speed_event_receiver,
                download_speed_event_receiver,
                log_event_receiver,
                stop_signal,
            },
        )
    }
    fn config(&self) -> Arc<C> {
        self.config.clone()
    }
    fn server_state(&self) -> Arc<ServerState> {
        self.server_state.clone()
    }

    async fn run<F1, Fut1, F2, Fut2>(
        self,
        create_listener: F1,
        connection_handler: F2,
    ) -> Result<(), CommonError>
    where
        F1: Fn(Arc<C>) -> Fut1 + Send + Sync + 'static,
        Fut1: Future<Output = Result<TcpListener, CommonError>> + Send + 'static,
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
        publish_server_log_event(
            &self.log_event_sender,
            LogEventLevel::Info,
            format!("Server listening on port: {}", config.server_port()),
        )
        .await;

        loop {
            tokio::select! {
                _ = self.stop_signal.cancelled()=>{
                    return Ok(())
                }
                accept_result=listener.accept()=>{
                    let (tcp_stream, socket_address) = match accept_result {
                        Ok(agent_tcp_accept_result) => agent_tcp_accept_result,
                        Err(e) => {
                            publish_server_log_event(
                                &self.log_event_sender,
                                LogEventLevel::Error,
                                format!("Failed to accept connection with IPv4 on port: {}", e),
                            )
                            .await;
                            continue;
                        }
                    };
                    publish_server_log_event(
                        &self.log_event_sender,
                        LogEventLevel::Info,
                        format!("Accept connection: {}", socket_address),
                    )
                    .await;
                    tcp_stream.set_nodelay(true)?;
                    let config = config.clone();
                    let server_state = server_state.clone();
                    let connection_handler = connection_handler.clone();
                    let log_event_sender = self.log_event_sender.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            connection_handler(config, server_state, tcp_stream, socket_address)
                                .await
                        {
                            publish_server_log_event(
                                &log_event_sender,
                                LogEventLevel::Error,
                                format!(
                                    "Fail to handle connection [{}] because of error: {e:?}",
                                    socket_address
                                ),
                            )
                            .await;
                        }
                    });
                }
            }
        }
    }
}
