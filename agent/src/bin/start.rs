use clap::Parser;
use ppaass_agent::start_server;
use ppaass_agent::AgentConfig;
use ppaass_agent::Command;
use ppaass_common::config::ServerConfig;
use ppaass_common::init_logger;
use ppaass_common::user::repo::fs::{
    FileSystemUserInfoRepository, FsAgentUserInfoContent, USER_INFO_ADDITION_INFO_PROXY_SERVERS,
};
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Builder;
use tracing::error;
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::parse();
    let config_file_path = command
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<AgentConfig>(&config_file_content)?);
    let log_dir = command.log_dir.unwrap_or(config.log_dir().clone());
    let _log_guard = init_logger(&log_dir, config.log_name_prefix(), config.max_log_level())?;
    let user_repo = Arc::new(FileSystemUserInfoRepository::new::<
        FsAgentUserInfoContent,
        _,
    >(config.user_dir(), |user_info, content| {
        user_info.add_additional_info(
            USER_INFO_ADDITION_INFO_PROXY_SERVERS,
            content.proxy_servers().to_owned(),
        );
    })?);
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(config.worker_thread_number())
        .build()?;
    runtime.block_on(async move {
        if let Err(e) = start_server(config, &user_repo).await {
            error!("Fail to start agent server: {e:?}")
        }
    });

    Ok(())
}
