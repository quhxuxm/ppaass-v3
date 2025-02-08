use anyhow::Result;
use clap::Parser;
use ppaass_common::config::RsaCryptoRepoConfig;
use proxy_tool::command::{ToolCommand, ToolSubCommand};
use proxy_tool::config::ProxyToolConfig;
use proxy_tool::crypto::{generate_agent_key_pairs, generate_proxy_key_pairs};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::sync::Arc;
const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

const DEFAULT_SEND_TO_AGENT_DIR: &str = "send_to_agent";
fn main() -> Result<()> {
    let command = ToolCommand::parse();
    let config_file_path = command
        .proxy_config_file
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<ProxyToolConfig>(&config_file_content)?);
    match command.sub_command {
        ToolSubCommand::GenerateRsa {
            authentication,
            agent_rsa_dir,
        } => {
            println!(
                "Begin to generate proxy RSA key for [{authentication}] in [{:?}]",
                config.rsa_dir()
            );
            generate_proxy_key_pairs(config.rsa_dir(), &authentication)?;
            println!(
                "Begin to generate agent RSA key for [{authentication}] in [{:?}], please send these file to agent user.",
                config.rsa_dir()
            );
            generate_agent_key_pairs(
                &agent_rsa_dir.unwrap_or(Path::new(DEFAULT_SEND_TO_AGENT_DIR).to_owned()),
                &authentication,
            )?;
        }
    }
    Ok(())
}
