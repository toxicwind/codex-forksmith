mod commands;
mod fs_config;
mod git;

use std::ffi::OsString;

use anyhow::Result;
use clap::{error::ErrorKind, Parser, Subcommand};

use commands::{build, run as run_cmd, status, sync};
use fs_config::ForksmithConfig;

#[derive(Parser, Debug)]
#[command(
    name = "codex-forksmith",
    version,
    about = "Forksmith v2 control plane"
)]
struct Cli {
    /// Force the loader to run `codex status` instead of launching the binary
    #[arg(long = "loader-status", action = clap::ArgAction::SetTrue)]
    loader_status: bool,
    /// Force the loader to run `codex sync` instead of launching the binary
    #[arg(long = "loader-sync", action = clap::ArgAction::SetTrue)]
    loader_sync: bool,
    /// Preview the sync step when combined with --loader-sync
    #[arg(
        long = "loader-sync-dry-run",
        action = clap::ArgAction::SetTrue,
        requires = "loader_sync"
    )]
    loader_sync_dry_run: bool,
    /// Force the loader to run `codex build` instead of launching the binary
    #[arg(long = "loader-build", action = clap::ArgAction::SetTrue)]
    loader_build: bool,
    /// Print the loader-specific usage banner
    #[arg(long = "loader-help", action = clap::ArgAction::SetTrue)]
    loader_help: bool,
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
    let raw_args_os: Vec<OsString> = std::env::args_os().collect();
    let raw_args: Vec<String> = raw_args_os
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    match Cli::try_parse_from(&raw_args_os) {
        Ok(cli) => handle_cli(cli),
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                err.print()?;
                Ok(())
            }
            ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand => run_passthrough(&raw_args),
            _ => Err(err.into()),
        },
    }
}

fn handle_cli(cli: Cli) -> Result<()> {
    if cli.loader_help {
        print_top_level_help();
        return Ok(());
    }

    let loader_flags = [cli.loader_status, cli.loader_sync, cli.loader_build]
        .into_iter()
        .filter(|flag| *flag)
        .count();

    if loader_flags > 1 {
        anyhow::bail!("only one --loader-* flag can be used at a time");
    }

    if loader_flags > 0 && cli.command.is_some() {
        anyhow::bail!("--loader-* flags cannot be combined with other codex commands");
    }

    if cli.loader_status {
        let cfg = ForksmithConfig::load_default()?;
        return status::run(&cfg);
    }
    if cli.loader_sync {
        let cfg = ForksmithConfig::load_default()?;
        return sync::run(&cfg, cli.loader_sync_dry_run);
    }
    if cli.loader_build {
        let cfg = ForksmithConfig::load_default()?;
        return build::run(&cfg);
    }

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
            let cfg = ForksmithConfig::load_default()?;
            run_cmd::run(&cfg, &[])
        }
    }
}

fn run_passthrough(raw_args: &[String]) -> Result<()> {
    let run_args = if raw_args.len() > 1 {
        raw_args[1..].to_vec()
    } else {
        Vec::new()
    };

    let cfg = ForksmithConfig::load_default()?;
    run_cmd::run(&cfg, &run_args)
}

fn print_top_level_help() {
    println!("Forksmith v2 control plane\n");
    println!("Common workflows:");
    println!("  codex status             # inspect workspace + vendor state");
    println!("  codex sync               # refresh remotes (add --dry-run to preview)");
    println!("  codex build              # build vendor/codex binary (cargo --profile release)");
    println!(
        "  codex resume             # run the codex binary (shorthand for `codex run -- resume`)"
    );
    println!("  codex --loader-sync      # invoke sync via loader passthrough");
    println!("  codex --loader-status    # invoke status via loader passthrough");
    println!("  codex --loader-build     # invoke build via loader passthrough\n");
    println!("For full help: codex --help or codex help <command>.");
}
