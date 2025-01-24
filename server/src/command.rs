use clap::Parser;
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Command {
    /// The configuration file path of the server
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// The rsa directory path of the server
    #[arg(short, long)]
    pub rsa: Option<PathBuf>,
    /// The log directory path of the server
    #[arg(short, long)]
    pub log_dir: Option<PathBuf>,
}
