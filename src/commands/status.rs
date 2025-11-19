use std::path::Path;

use anyhow::Result;

use crate::fs_config::ForksmithConfig;
use crate::git;

pub fn run(cfg: &ForksmithConfig) -> Result<()> {
    let repo = &cfg.repo_path;
    git::ensure_repo(repo)?;
    let branch = git::current_branch(repo)?;
    let clean = git::is_clean(repo)?;
    let local_ref = format!("{}/{}", cfg.local_remote, cfg.local_branch);
    let upstream_ref = format!("{}/{}", cfg.upstream_remote, cfg.upstream_branch);
    let head = git::head_commit(repo)?;
    let (ahead_local, behind_local) = divergence(repo, "HEAD", &local_ref)?;
    let (ahead_upstream, behind_upstream) = divergence(repo, "HEAD", &upstream_ref)?;
    let binary_path = cfg.repo_binary_path();
    let binary_exists = binary_path.exists();

    println!("workspace     = {}", cfg.workspace_root.display());
    println!("repo          = {}", repo.display());
    println!("build_dir     = {}", cfg.build_workspace.display());
    println!("branch        = {}", branch);
    println!("head          = {}", head);
    println!("clean         = {}", clean);
    println!(
        "local_ref     = {} (ahead {}, behind {})",
        local_ref, ahead_local, behind_local
    );
    println!(
        "upstream_ref  = {} (ahead {}, behind {})",
        upstream_ref, ahead_upstream, behind_upstream
    );
    println!(
        "binary        = {} (exists={})",
        binary_path.display(),
        binary_exists
    );
    Ok(())
}

fn divergence(repo: &Path, base: &str, other: &str) -> Result<(u32, u32)> {
    match git::divergence(repo, base, other) {
        Ok(v) => Ok(v),
        Err(err) => {
            println!("warning: unable to compute divergence for {other}: {err:#}");
            Ok((0, 0))
        }
    }
}
