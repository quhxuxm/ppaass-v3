use clap::Parser;
use command::Command;
use ppaass_agent_core::start_server;
use ppaass_agent_core::AgentConfig;
use ppaass_common::init_logger;
use ppaass_common::user::repo::create_fs_user_repository;
use std::error::Error as StdError;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Builder;
use tracing::error;
pub mod command;
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

fn main() -> Result<(), Box<dyn StdError>> {
    let command = Command::parse();
    let config_file_path = command
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<AgentConfig>(&config_file_content)?);
    let log_dir = command.log_dir.unwrap_or(config.log_dir.clone());
    let _log_guard = init_logger(&log_dir, &config.log_name_prefix, &config.max_log_level)?;
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(config.worker_thread_number)
        .build()?;
    runtime.block_on(async move {
        let fs_user_repo = match create_fs_user_repository(
            &config.user_dir,
            config.user_info_repository_refresh_interval,
        )
        .await
        {
            Ok(fs_user_repo) => fs_user_repo,
            Err(e) => {
                error!("Fail to start agent server when create user info repository: {e:?}");
                return;
            }
        };
        let user_repo = Arc::new(fs_user_repo);
        if let Err(e) = start_server(config.clone(), user_repo.clone()).await {
            error!("Fail to start agent server: {e:?}");
        }
    });

    Ok(())
}
