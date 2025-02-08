use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct ToolCommand {
    #[arg(short, long)]
    pub config_file: PathBuf,
    #[clap(subcommand)]
    pub sub_command: ToolSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ToolSubCommand {
    #[command(name = "gen-rsa")]
    GenerateRsa {
        #[arg(short, long)]
        authentication: String,
    },
}
