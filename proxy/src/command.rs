use clap::Parser;
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Command {
    /// The configuration file path of the proxy
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// The agent_rsa directory path of the proxy
    #[arg(short, long)]
    pub agent_rsa_dir: Option<PathBuf>,
    /// The forward agent_rsa directory path of the proxy
    #[arg(short, long)]
    pub forward_rsa: Option<PathBuf>,
    /// The log directory path of the proxy
    #[arg(short, long)]
    pub log_dir: Option<PathBuf>,
}
