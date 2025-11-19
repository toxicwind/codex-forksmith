use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::fs_config::ForksmithConfig;

pub fn run(cfg: &ForksmithConfig, args: &[String]) -> Result<()> {
    let binary = cfg.repo_binary_path();
    if !binary.exists() {
        anyhow::bail!(
            "{} missing; run `codex-forksmith build` first",
            binary.display()
        );
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
