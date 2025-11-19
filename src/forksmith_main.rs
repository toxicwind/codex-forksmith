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
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show git + fork status for vendor/codex
    Status,
    /// Fetch remotes and prep for merges
    Sync {
        /// Show what would happen without mutating the repo
        #[arg(long, action = clap::ArgAction::SetTrue)]
        dry_run: bool,
    },
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
    match cli.command {
        Some(command) => {
            let cfg = ForksmithConfig::load_default()?;
            match command {
                Commands::Status => status::run(&cfg),
                Commands::Sync { dry_run } => sync::run(&cfg, dry_run),
                Commands::Build => build::run(&cfg),
                Commands::Run { args } => run_cmd::run(&cfg, &args),
            }
        }
        None => {
            print_top_level_help();
            Ok(())
        }
    }
}

fn print_top_level_help() {
    println!("Forksmith v2 control plane\n");
    println!("Common workflows:");
    println!("  codex status         # inspect workspace + vendor state");
    println!("  codex sync           # refresh remotes (add --dry-run to preview)");
    println!("  codex build          # build vendor/codex binary (cargo --profile release)");
    println!("  codex run -- <args>  # run the codex binary with passthrough args\n");
    println!("For full help: codex --help or codex help <command>.");
}
