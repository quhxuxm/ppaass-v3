use clap::Parser;
use command::Command;
use ppaass_common::error::CommonError;
use ppaass_common::server::{CommonServer, Server, ServerListener, ServerState};
use ppaass_common::user::repo::create_fs_user_repository;
use ppaass_common::user::repo::fs::{
    FileSystemUserInfoRepository, FsProxyUserInfoContent, USER_INFO_ADDITION_INFO_EXPIRED_DATE_TIME,
};
use ppaass_common::user::UserInfoRepository;
use ppaass_common::{init_logger, ProxyTcpConnectionPool};
pub use ppaass_proxy_core::config::*;
use ppaass_proxy_core::tunnel::handle_agent_connection;
use ppaass_proxy_core::user::ForwardProxyUserRepository;
use std::fs::read_to_string;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{net::TcpListener, runtime::Builder};
use tracing::{debug, error, trace};
pub mod command;
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

async fn create_server_listener(config: Arc<ProxyConfig>) -> Result<ServerListener, CommonError> {
    if config.ip_v6() {
        debug!(
            "Starting server listener with IPv6 on port: {}",
            config.server_port()
        );
        Ok(ServerListener::TcpListener(
            TcpListener::bind(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                config.server_port(),
            ))
            .await?,
        ))
    } else {
        debug!(
            "Starting server listener with IPv4 on port: {}",
            config.server_port()
        );
        Ok(ServerListener::TcpListener(
            TcpListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                config.server_port(),
            ))
            .await?,
        ))
    }
}

async fn start_server<T: UserInfoRepository + Send + Sync + 'static>(
    config: Arc<ProxyConfig>,
    agent_user_repo: Arc<T>,
) -> Result<(), CommonError> {
    let mut server_state = ServerState::new();
    server_state.add_value(agent_user_repo.clone());
    if let Some(forward_config) = config.forward() {
        let forward_config = Arc::new(forward_config.clone());
        let forward_fs_user_repo = ForwardProxyUserRepository::new(
            create_fs_user_repository(
                config.user_dir(),
                config.user_info_repository_refresh_interval(),
            )
            .await?,
        );
        let forward_proxy_user_repo = Arc::new(forward_fs_user_repo);
        let (username, forward_proxy_user_info) = {
            let username = forward_config.username();
            let user_info =
                forward_proxy_user_repo
                    .get_user(username)
                    .await?
                    .ok_or(CommonError::Other(format!(
                        "Can not get forward user info from repository: {username}"
                    )))?;
            (username, user_info)
        };
        server_state.add_value((username.to_owned(), forward_proxy_user_info.clone()));
        if forward_config.connection_pool().is_some() {
            let proxy_tcp_connection_pool = ProxyTcpConnectionPool::new(
                forward_config.clone(),
                username,
                forward_proxy_user_info,
            )
            .await?;
            server_state.add_value(Arc::new(proxy_tcp_connection_pool));
        }
    }

    let server = CommonServer::new(config.clone(), server_state);
    server
        .run(create_server_listener, handle_agent_connection)
        .await?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::parse();
    let config_file_path = command
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<ProxyConfig>(&config_file_content)?);
    let log_dir = command.log_dir.unwrap_or(config.log_dir().clone());
    let _log_guard = init_logger(&log_dir, config.log_name_prefix(), config.max_log_level())?;

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(config.worker_thread_number())
        .build()?;
    runtime.block_on(async move {
        let user_dir = command
            .agent_rsa_dir
            .unwrap_or(config.user_dir().to_owned());
        debug!("Rsa directory of the proxy server: {user_dir:?}");
        let fs_user_repo = match FileSystemUserInfoRepository::new::<FsProxyUserInfoContent, _, _>(
            config.user_info_repository_refresh_interval(),
            &user_dir,
            |user_info, content| async move {
                if let Some(expired_date_time) = content.expired_date_time() {
                    let mut user_info = user_info.write().await;
                    user_info.add_additional_info(
                        USER_INFO_ADDITION_INFO_EXPIRED_DATE_TIME,
                        expired_date_time.to_owned(),
                    );
                }
            },
        )
        .await
        {
            Ok(fs_user_repo) => fs_user_repo,
            Err(e) => {
                error!("Fail to start proxy server when create user info repository: {e:?}");
                return;
            }
        };
        let rsa_crypto_repo = Arc::new(fs_user_repo);
        trace!("Success to create agent_user crypto repo: {rsa_crypto_repo:?}");
        if let Err(e) = start_server(config.clone(), rsa_crypto_repo.clone()).await {
            error!("Fail to start proxy server: {e:?}");
        }
    });
    Ok(())
}
