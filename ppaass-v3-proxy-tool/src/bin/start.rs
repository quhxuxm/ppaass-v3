use anyhow::Result;
use clap::Parser;
use ppaass_v3_proxy_tool::command::{ToolCommand, ToolSubCommand};
use ppaass_v3_proxy_tool::config::ProxyToolConfig;
use ppaass_v3_proxy_tool::handler::generate_user::{generate_user, GenerateUserHandlerArgument};
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
const DEFAULT_CONFIG_FILE: &str = "ppaass-v3-agent-ppaass-v3-proxy-resources/config.toml";
fn main() -> Result<()> {
    let command = ToolCommand::parse();
    let config_file_path = command
        .proxy_config_file
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<ProxyToolConfig>(&config_file_content)?);
    match command.sub_command {
        ToolSubCommand::GenerateUser {
            username,
            agent_rsa_dir,
            temp_dir,
            expire_after_days,
            proxy_servers,
        } => generate_user(
            config.as_ref(),
            GenerateUserHandlerArgument {
                username,
                agent_rsa_dir,
                temp_dir,
                expire_after_days,
                proxy_servers,
            },
        )?,
    }
    Ok(())
}
