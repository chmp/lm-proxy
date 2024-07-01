use std::path::PathBuf;

use anyhow::Result;
use argh::FromArgs;

mod serve;

#[derive(FromArgs, PartialEq, Debug)]
/// llm_proxy - a proxy for LLM models
pub struct Args {
    #[argh(subcommand)]
    command: Command,
}

impl Args {
    pub fn run(self) -> Result<()> {
        self.command.run()
    }
}

#[derive(FromArgs, Debug, PartialEq)]
#[argh(subcommand)]
pub enum Command {
    Serve(ServeCommand),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Serve(cmd) => cmd.run(),
        }
    }
}

#[derive(FromArgs, PartialEq, Debug)]
/// Serve the configured moudles
#[argh(subcommand, name = "serve")]
pub struct ServeCommand {
    /// the toml config file
    #[argh(positional)]
    pub config: PathBuf,
}
