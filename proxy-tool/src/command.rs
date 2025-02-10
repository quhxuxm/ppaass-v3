use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct ToolCommand {
    #[arg(short, long)]
    pub proxy_config_file: Option<PathBuf>,
    #[clap(subcommand)]
    pub sub_command: ToolSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ToolSubCommand {
    #[command(name = "gen-user")]
    GenerateUser {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        agent_rsa_dir: Option<PathBuf>,
        #[arg(short, long)]
        temp_dir: Option<PathBuf>,
        #[arg(short, long)]
        expire_after_days: Option<i64>,
        #[arg(short, long)]
        proxy_servers: Option<Vec<String>>,
    },
}
