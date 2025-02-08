use crate::config::ProxyToolConfig;
use crate::crypto::{generate_agent_key_pairs, generate_proxy_key_pairs};
use anyhow::Result;
use ppaass_common::config::RsaCryptoRepoConfig;
use std::path::{Path, PathBuf};

const DEFAULT_SEND_TO_AGENT_DIR: &str = "send_to_agent";
pub struct GenerateRsaHandlerArgument {
    pub authentication: String,
    pub agent_rsa_dir: Option<PathBuf>,
}
pub fn generate_rsa(config: &ProxyToolConfig, arg: GenerateRsaHandlerArgument) -> Result<()> {
    println!(
        "Begin to generate proxy RSA key for [{}] in [{:?}]",
        arg.authentication,
        config.rsa_dir()
    );
    generate_proxy_key_pairs(config.rsa_dir(), &arg.authentication)?;
    println!(
        "Begin to generate agent RSA key for [{}] in [{:?}], please send these file to agent user.",
        arg.authentication,
        config.rsa_dir()
    );
    generate_agent_key_pairs(
        &arg.agent_rsa_dir
            .unwrap_or(Path::new(DEFAULT_SEND_TO_AGENT_DIR).to_owned()),
        &arg.authentication,
    )?;
    Ok(())
}
