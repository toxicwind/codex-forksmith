use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};

use crate::fs_config::ForksmithConfig;
use crate::git;

pub fn run(cfg: &ForksmithConfig, dry_run: bool) -> Result<()> {
    let repo = &cfg.repo_path;
    git::ensure_repo(repo)?;
    let clean = git::is_clean(repo)?;
    if !dry_run && !clean {
        bail!(
            "repo {} has local changes; commit or stash before syncing",
            repo.display()
        );
    }
    if dry_run && !clean {
        println!("(dry-run) repo has local changes; would require a clean tree before syncing");
    }
    let mut fetched = BTreeSet::new();
    for remote in [&cfg.local_remote, &cfg.upstream_remote] {
        if git::has_remote(repo, remote)? {
            println!("fetching {remote}...");
            git::fetch(repo, remote).with_context(|| format!("fetching {remote}"))?;
            fetched.insert(remote.to_string());
        } else {
            println!("remote {remote} missing; skipping fetch");
        }
    }

    let branch = git::current_branch(repo)?;
    let upstream_ref = format!("{}/{}", cfg.upstream_remote, cfg.upstream_branch);
    let local_ref = format!("{}/{}", cfg.local_remote, cfg.local_branch);
    println!("current branch: {branch}");

    let (_, behind_upstream) = git::divergence(repo, "HEAD", &upstream_ref)?;
    let mut ff_applied = false;
    if behind_upstream > 0 {
        if dry_run {
            println!("(dry-run) would fast-forward to {upstream_ref} (+{behind_upstream})");
        } else {
            println!("fast-forwarding to {upstream_ref} ({behind_upstream} commits)...");
            git::fast_forward(repo, &upstream_ref)?;
            ff_applied = true;
        }
    } else {
        println!("already up to date with {upstream_ref}");
    }

    let (_, behind_local) = git::divergence(repo, "HEAD", &local_ref)?;
    if behind_local > 0 {
        println!("local remote {local_ref} is ahead by {behind_local} commit(s); push soon");
    } else {
        println!("local remote {local_ref} matches HEAD");
    }

    let upstream_behind_after = if ff_applied { 0 } else { behind_upstream };
    println!(
        "SYNC_RESULT dry_run={} fetched={} ff_applied={} behind_local={} behind_upstream={}",
        dry_run,
        fetched.into_iter().collect::<Vec<_>>().join(","),
        ff_applied,
        behind_local,
        upstream_behind_after
    );
    Ok(())
}
