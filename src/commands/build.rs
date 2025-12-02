use std::{env, process::Command};

use anyhow::{Context, Result};

use crate::fs_config::ForksmithConfig;
use crate::git;
use which::which;

pub fn run(cfg: &ForksmithConfig) -> Result<()> {
    let repo = &cfg.repo_path;
    git::ensure_repo(repo)?;
    if !git::is_clean(repo)? {
        println!(
            "warning: repository {} has local changes; building anyway",
            repo.display()
        );
    }
    println!(
        "building codex in {} (profile {})",
        cfg.build_workspace.display(),
        cfg.build_profile
    );
    let mut command = Command::new("cargo");
    command
        .args(["build", "--profile", &cfg.build_profile])
        .current_dir(&cfg.build_workspace);
    configure_rustc_wrapper(&mut command);
    let status = command.status().context("running cargo build")?;
    if !status.success() {
        anyhow::bail!("cargo build failed");
    }
    let binary = cfg.repo_binary_path();
    if !binary.exists() {
        anyhow::bail!("expected binary {} missing after build", binary.display());
    }
    println!("built {}", binary.display());
    Ok(())
}

fn configure_rustc_wrapper(command: &mut Command) {
    if env::var_os("RUSTC_WRAPPER").is_some() {
        return;
    }
    if let Ok(sccache_path) = which("sccache") {
        command.env("RUSTC_WRAPPER", &sccache_path);
        println!("  using sccache via {}", sccache_path.display());
    }
}
