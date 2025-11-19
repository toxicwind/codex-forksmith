use std::path::Path;

use anyhow::{bail, Result};

use crate::fs_config::ForksmithConfig;
use crate::git;

pub fn run(cfg: &ForksmithConfig) -> Result<()> {
    let report = StatusReport::gather(cfg)?;
    report.print();
    if report.should_fail() {
        bail!("status check failed; resolve issues above");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub workspace_root: String,
    pub repo: String,
    pub build_dir: String,
    pub branch: String,
    pub head: String,
    pub clean: bool,
    pub has_conflicts: bool,
    pub tracked: usize,
    pub untracked: usize,
    pub local_ref: String,
    pub upstream_ref: String,
    pub local_ahead: u32,
    pub local_behind: u32,
    pub upstream_ahead: u32,
    pub upstream_behind: u32,
    pub binary_path: String,
    pub binary_exists: bool,
}

impl StatusReport {
    pub fn gather(cfg: &ForksmithConfig) -> Result<Self> {
        let repo = &cfg.repo_path;
        git::ensure_repo(repo)?;
        let branch = git::current_branch(repo)?;
        let clean = git::is_clean(repo)?;
        let has_conflicts = git::has_unmerged_paths(repo)?;
        let snapshot = git::status_snapshot(repo)?;
        let local_ref = format!("{}/{}", cfg.local_remote, cfg.local_branch);
        let upstream_ref = format!("{}/{}", cfg.upstream_remote, cfg.upstream_branch);
        let head = git::head_commit(repo)?;
        let (ahead_local, behind_local) = divergence(repo, "HEAD", &local_ref)?;
        let (ahead_upstream, behind_upstream) = divergence(repo, "HEAD", &upstream_ref)?;
        let binary_path = cfg.repo_binary_path();
        let binary_exists = binary_path.exists();
        Ok(Self {
            workspace_root: cfg.workspace_root.display().to_string(),
            repo: repo.display().to_string(),
            build_dir: cfg.build_workspace.display().to_string(),
            branch,
            head,
            clean,
            has_conflicts,
            tracked: snapshot.tracked,
            untracked: snapshot.untracked,
            local_ref,
            upstream_ref,
            local_ahead: ahead_local,
            local_behind: behind_local,
            upstream_ahead: ahead_upstream,
            upstream_behind: behind_upstream,
            binary_path: binary_path.display().to_string(),
            binary_exists,
        })
    }

    pub fn print(&self) {
        println!("workspace     = {}", self.workspace_root);
        println!("repo          = {}", self.repo);
        println!("build_dir     = {}", self.build_dir);
        println!("branch        = {}", self.branch);
        println!("head          = {}", self.head);
        println!("clean         = {}", self.clean);
        println!(
            "dirty_counts  = tracked {} untracked {}",
            self.tracked, self.untracked
        );
        if self.has_conflicts {
            println!("conflicts     = true (resolve git merge conflicts)");
        }
        println!(
            "local_ref     = {} (ahead {}, behind {})",
            self.local_ref, self.local_ahead, self.local_behind
        );
        println!(
            "upstream_ref  = {} (ahead {}, behind {})",
            self.upstream_ref, self.upstream_ahead, self.upstream_behind
        );
        println!(
            "binary        = {} (exists={})",
            self.binary_path, self.binary_exists
        );
    }

    pub fn should_fail(&self) -> bool {
        self.has_conflicts || !self.binary_exists
    }
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

#[cfg(test)]
mod tests {
    use super::StatusReport;

    #[test]
    fn status_report_flags_failures() {
        let mut report = sample_report();
        assert!(!report.should_fail());
        report.has_conflicts = true;
        assert!(report.should_fail());
        report.has_conflicts = false;
        report.binary_exists = false;
        assert!(report.should_fail());
    }

    fn sample_report() -> StatusReport {
        StatusReport {
            workspace_root: ".".into(),
            repo: "vendor/codex".into(),
            build_dir: "vendor/codex/codex-rs".into(),
            branch: "main".into(),
            head: "deadbeef".into(),
            clean: true,
            has_conflicts: false,
            tracked: 0,
            untracked: 0,
            local_ref: "origin/main".into(),
            upstream_ref: "upstream/main".into(),
            local_ahead: 0,
            local_behind: 0,
            upstream_ahead: 0,
            upstream_behind: 0,
            binary_path: "codex-rs/target/release/codex".into(),
            binary_exists: true,
        }
    }
}
