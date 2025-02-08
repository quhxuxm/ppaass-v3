use crate::command::{ToolCommand, ToolSubCommand};
use anyhow::Result;
use clap::Parser;
mod command;
mod crypto;
fn main() -> Result<()> {
    let command = ToolCommand::parse();
    let config = command.config_file;
    match command.sub_command {
        ToolSubCommand::GenerateRsa { .. } => {}
    }
    Ok(())
}
