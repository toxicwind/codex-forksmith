use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::commands::build;
use crate::fs_config::ForksmithConfig;

pub fn run(cfg: &ForksmithConfig, args: &[String]) -> Result<()> {
    let binary = cfg.repo_binary_path();
    if !binary.exists() {
        println!(
            "binary {} missing; building via `codex build` before running",
            binary.display()
        );
        build::run(cfg)?;
    }
    let mut cmd = Command::new(&binary);
    cmd.args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd
        .status()
        .with_context(|| format!("launching {}", binary.display()))?;
    if !status.success() {
        anyhow::bail!("codex exited with {status}");
    }
    Ok(())
}
