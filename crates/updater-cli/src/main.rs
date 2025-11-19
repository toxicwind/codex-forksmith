use std::env;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use codex_core::{run_update, UpdateOptions, UpdateSummary};
use codex_registry::RegistryStore;
use serde::Serialize;
use tracing_subscriber::{fmt, EnvFilter};

fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Commands::Update(args) => cmd_update(args),
        Commands::Registry(cmd) => cmd_registry(cmd),
        Commands::Doctor(args) => cmd_doctor(args),
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Codex Forksmith experimental orchestrator",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Update(UpdateArgs),
    Registry(RegistryArgs),
    Doctor(DoctorArgs),
}

#[derive(Args, Debug)]
struct UpdateArgs {
    #[arg(long)]
    workspace: Option<Utf8PathBuf>,

    #[arg(long)]
    vendor_dir: Option<Utf8PathBuf>,

    #[arg(long)]
    registry: Option<Utf8PathBuf>,

    #[arg(long)]
    ast_rules: Option<Utf8PathBuf>,

    #[arg(long)]
    cocci_rules: Option<Utf8PathBuf>,

    #[arg(long, default_value = "main")]
    branch: String,

    #[arg(long)]
    output_zip: Option<Utf8PathBuf>,

    #[arg(long)]
    skip_cargo_check: bool,

    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct RegistryArgs {
    #[arg(long)]
    registry: Option<Utf8PathBuf>,

    #[command(subcommand)]
    command: RegistryCommand,
}

#[derive(Subcommand, Debug)]
enum RegistryCommand {
    List,
    Enable { id: String },
    Disable { id: String },
}

#[derive(Args, Debug)]
struct DoctorArgs {
    #[arg(long)]
    workspace: Option<Utf8PathBuf>,
}

fn cmd_update(args: UpdateArgs) -> Result<()> {
    let workspace = args
        .workspace
        .or_else(default_workspace)
        .unwrap_or_else(|| Utf8PathBuf::from_path_buf(env::current_dir().unwrap()).unwrap());
    let vendor_dir = args
        .vendor_dir
        .unwrap_or_else(|| workspace.join("vendor/codex"));
    let registry_path = args
        .registry
        .unwrap_or_else(|| workspace.join("patch-registry/registry.json"));
    let ast_rules_dir = args.ast_rules;
    let cocci_rules_dir = args.cocci_rules;

    let summary = run_update(UpdateOptions {
        workspace_root: workspace.clone(),
        vendor_dir,
        registry_path,
        ast_rules_dir,
        coccinelle_rules_dir: cocci_rules_dir,
        upstream_branch: args.branch,
        cargo_check: !args.skip_cargo_check,
        output_zip: args.output_zip,
    })?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_summary(&summary);
    }
    Ok(())
}

fn cmd_registry(args: RegistryArgs) -> Result<()> {
    let workspace = default_workspace()
        .unwrap_or_else(|| Utf8PathBuf::from_path_buf(env::current_dir().unwrap()).unwrap());
    let path = args
        .registry
        .unwrap_or_else(|| workspace.join("patch-registry/registry.json"));
    let store = RegistryStore::new(path);
    let mut registry = store.load()?;
    match args.command {
        RegistryCommand::List => {
            for set in &registry.patch_sets {
                println!(
                    "{} [{}] enabled={} notes={:?}",
                    set.id, set.description, set.enabled, set.notes
                );
            }
        }
        RegistryCommand::Enable { id } => {
            registry.toggle(&id, true)?;
            store.save(&registry)?;
            println!("enabled {id}");
        }
        RegistryCommand::Disable { id } => {
            registry.toggle(&id, false)?;
            store.save(&registry)?;
            println!("disabled {id}");
        }
    }
    Ok(())
}

fn cmd_doctor(args: DoctorArgs) -> Result<()> {
    let workspace = args
        .workspace
        .or_else(default_workspace)
        .unwrap_or_else(|| Utf8PathBuf::from_path_buf(env::current_dir().unwrap()).unwrap());
    let checks = DoctorReport {
        workspace_exists: workspace.exists(),
        vendor_exists: workspace.join("vendor/codex").exists(),
        registry_exists: workspace.join("patch-registry/registry.json").exists(),
    };
    println!("{}", serde_json::to_string_pretty(&checks)?);
    Ok(())
}

fn default_workspace() -> Option<Utf8PathBuf> {
    let home = env::var("HOME").ok()?;
    let new_path = Utf8PathBuf::from(format!("{home}/development/codex-forksmith"));
    let legacy_path = Utf8PathBuf::from(format!("{home}/development/codex-patcher-updater"));
    if new_path.exists() {
        Some(new_path)
    } else if legacy_path.exists() {
        Some(legacy_path)
    } else {
        None
    }
}

fn print_summary(summary: &UpdateSummary) {
    println!("vendor before: {:?}", summary.vendor_rev_before);
    println!("vendor after : {:?}", summary.vendor_rev_after);
    if !summary.ast_notes.is_empty() {
        println!("ast-grep:");
        for note in &summary.ast_notes {
            println!("  - {note}");
        }
    }
    if !summary.cocci_notes.is_empty() {
        println!("coccinelle:");
        for note in &summary.cocci_notes {
            println!("  - {note}");
        }
    }
    println!("cargo check: {}", summary.cargo_check_passed);
    if !summary.warnings.is_empty() {
        println!("warnings:");
        for w in &summary.warnings {
            println!("  - {w}");
        }
    }
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    workspace_exists: bool,
    vendor_exists: bool,
    registry_exists: bool,
}
