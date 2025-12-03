use std::env;
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
    let final_args = append_default_cwd_arg(args)?;
    let mut cmd = Command::new(&binary);
    cmd.args(&final_args)
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

fn append_default_cwd_arg(args: &[String]) -> Result<Vec<String>> {
    if contains_cwd_flag(args) {
        return Ok(args.to_vec());
    }
    let cwd = env::current_dir().context("resolving current directory")?;
    let mut final_args = Vec::with_capacity(args.len() + 2);
    final_args.push("-C".to_string());
    final_args.push(cwd.display().to_string());
    final_args.extend_from_slice(args);
    Ok(final_args)
}

fn contains_cwd_flag(args: &[String]) -> bool {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-C" || arg == "--cd" {
            return true;
        }
        if arg.starts_with("--cd=") {
            return true;
        }
    }
    false
}
