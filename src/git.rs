use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

pub fn ensure_repo(repo: &Path) -> Result<()> {
    if !repo.exists() {
        anyhow::bail!("repo {} missing", repo.display());
    }
    if !repo.join(".git").exists() {
        anyhow::bail!("{} is not a git repository", repo.display());
    }
    Ok(())
}

pub fn run_git(repo: &Path, args: &[&str]) -> Result<String> {
    ensure_repo(repo)?;
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("running git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn current_branch(repo: &Path) -> Result<String> {
    run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn head_commit(repo: &Path) -> Result<String> {
    run_git(repo, &["rev-parse", "HEAD"])
}

pub fn is_clean(repo: &Path) -> Result<bool> {
    ensure_repo(repo)?;
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("checking git status in {}", repo.display()))?;
    if !status.status.success() {
        anyhow::bail!(
            "git status failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
    Ok(status.stdout.is_empty())
}

#[derive(Debug, Clone, Copy)]
pub struct StatusSnapshot {
    pub tracked: usize,
    pub untracked: usize,
}

pub fn status_snapshot(repo: &Path) -> Result<StatusSnapshot> {
    ensure_repo(repo)?;
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("checking git status in {}", repo.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut tracked = 0;
    let mut untracked = 0;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.starts_with("??") {
            untracked += 1;
        } else if !line.trim().is_empty() {
            tracked += 1;
        }
    }
    Ok(StatusSnapshot { tracked, untracked })
}

pub fn has_unmerged_paths(repo: &Path) -> Result<bool> {
    ensure_repo(repo)?;
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=U"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("checking unmerged paths in {}", repo.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "git diff --diff-filter=U failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

pub fn fetch(repo: &Path, remote: &str) -> Result<()> {
    run_git(repo, &["fetch", remote]).map(|_| ())
}

pub fn push(repo: &Path, remote: &str, branch: &str) -> Result<()> {
    run_git(repo, &["push", remote, branch]).map(|_| ())
}

pub fn has_remote(repo: &Path, remote: &str) -> Result<bool> {
    let output = run_git(repo, &["remote"])?;
    Ok(output.lines().any(|line| line.trim() == remote))
}

pub fn divergence(repo: &Path, base: &str, other: &str) -> Result<(u32, u32)> {
    let spec = format!("{base}...{other}");
    let output = run_git(repo, &["rev-list", "--left-right", "--count", &spec])?;
    let mut parts = output.split_whitespace();
    let left = parts
        .next()
        .ok_or_else(|| anyhow!("unexpected rev-list output"))?
        .parse::<u32>()
        .with_context(|| format!("parsing left count from {}", output))?;
    let right = parts
        .next()
        .ok_or_else(|| anyhow!("unexpected rev-list output"))?
        .parse::<u32>()
        .with_context(|| format!("parsing right count from {}", output))?;
    Ok((left, right))
}

pub fn fast_forward(repo: &Path, target: &str) -> Result<()> {
    run_git(repo, &["merge", "--ff-only", target]).map(|_| ())
}
