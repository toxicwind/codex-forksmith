mod config;
mod dev;
mod engines;
mod process;
mod registry;
mod runner;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use runner::UpdateOptions;

#[derive(Parser, Debug)]
#[command(
    name = "codex-forksmith",
    version,
    about = "Sovereign fork-stage helper for Codex; applies registry-driven patch sets atop vendor/codex"
)]
struct Cli {
    /// Workspace root (repo containing vendor/codex and codex-forksmith.toml)
    #[arg(long, global = true, default_value = ".")]
    root: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Pull upstream, apply patches, update registry, and build
    Update(UpdateArgs),
    /// Check environment, tools, and vendor repo state
    Doctor,
    /// Registry management commands
    #[command(subcommand)]
    Registry(RegistryCmd),
    /// Developer utilities (formatting, linting, etc.)
    #[command(subcommand)]
    Dev(DevCommand),
}

#[derive(Args, Debug)]
struct UpdateArgs {
    /// Do not write changes; just report what would happen
    #[arg(long)]
    dry_run: bool,
    /// Skip cargo build even when not in dry-run mode
    #[arg(long)]
    skip_build: bool,
    /// Emit machine-readable JSON summary
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum RegistryCmd {
    /// List registered patch sets
    List,
    /// Show detailed information for a patch set
    Explain {
        /// Patch-set id (e.g. astgrep:increase-max-output-tokens)
        id: String,
    },
    /// Enable a patch-set by id
    Enable {
        #[arg(value_name = "ID")]
        id: String,
    },
    /// Disable a patch-set by id
    Disable {
        #[arg(value_name = "ID")]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum DevCommand {
    /// Watch the repo and auto-run `cargo fmt` + `cargo clippy -D warnings`
    Watch,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = PathBuf::from(cli.root);

    match cli.command {
        Command::Update(args) => {
            let opts = UpdateOptions::new(args.dry_run, args.skip_build, args.json);
            runner::run_update(&root, opts)
        }
        Command::Doctor => runner::run_health(&root),
        Command::Registry(RegistryCmd::List) => runner::run_list_patches(&root),
        Command::Registry(RegistryCmd::Explain { id }) => runner::run_explain_patch(&root, &id),
        Command::Registry(RegistryCmd::Enable { id }) => runner::run_toggle_patch(&root, &id, true),
        Command::Registry(RegistryCmd::Disable { id }) => {
            runner::run_toggle_patch(&root, &id, false)
        }
        Command::Dev(DevCommand::Watch) => dev::run_watch(&root),
    }
}
