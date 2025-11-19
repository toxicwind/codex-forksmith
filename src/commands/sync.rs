use anyhow::{bail, Context, Result};

use crate::fs_config::ForksmithConfig;
use crate::git;

pub fn run(cfg: &ForksmithConfig) -> Result<()> {
    let repo = &cfg.repo_path;
    git::ensure_repo(repo)?;
    if !git::is_clean(repo)? {
        bail!(
            "repo {} has local changes; commit or stash before syncing",
            repo.display()
        );
    }
    for remote in [&cfg.local_remote, &cfg.upstream_remote] {
        if git::has_remote(repo, remote)? {
            println!("fetching {remote}...");
            git::fetch(repo, remote).with_context(|| format!("fetching {remote}"))?;
        } else {
            println!("remote {remote} missing; skipping fetch");
        }
    }

    let branch = git::current_branch(repo)?;
    let upstream_ref = format!("{}/{}", cfg.upstream_remote, cfg.upstream_branch);
    let local_ref = format!("{}/{}", cfg.local_remote, cfg.local_branch);
    println!("current branch: {branch}");

    let (_, behind_upstream) = git::divergence(repo, "HEAD", &upstream_ref)?;
    if behind_upstream > 0 {
        println!("fast-forwarding to {upstream_ref} ({behind_upstream} commits)...");
        git::fast_forward(repo, &upstream_ref)?;
    } else {
        println!("already up to date with {upstream_ref}");
    }

    let (_, behind_local) = git::divergence(repo, "HEAD", &local_ref)?;
    if behind_local > 0 {
        println!("local remote {local_ref} is ahead by {behind_local} commit(s); push soon");
    } else {
        println!("local remote {local_ref} matches HEAD");
    }

    Ok(())
}
