use std::process::Command;

use anyhow::{Context, Result};

use crate::fs_config::ForksmithConfig;
use crate::git;

pub fn run(cfg: &ForksmithConfig) -> Result<()> {
    let repo = &cfg.repo_path;
    git::ensure_repo(repo)?;
    println!(
        "building codex in {} (profile {})",
        cfg.build_workspace.display(),
        cfg.build_profile
    );
    let status = Command::new("cargo")
        .args(["build", "--profile", &cfg.build_profile])
        .current_dir(&cfg.build_workspace)
        .status()
        .context("running cargo build")?;
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
