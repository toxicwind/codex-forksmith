mod commands;
mod fs_config;
mod git;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::{build, run as run_cmd, status, sync};
use fs_config::ForksmithConfig;

#[derive(Parser, Debug)]
#[command(
    name = "codex-forksmith",
    version,
    about = "Forksmith v2 control plane"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show git + fork status for vendor/codex
    Status,
    /// Fetch remotes and prep for merges
    Sync,
    /// Build codex inside vendor/codex
    Build,
    /// Run the codex binary with passthrough args
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = ForksmithConfig::load_default()?;
    match cli.command {
        Commands::Status => status::run(&cfg),
        Commands::Sync => sync::run(&cfg),
        Commands::Build => build::run(&cfg),
        Commands::Run { args } => run_cmd::run(&cfg, &args),
    }
}
