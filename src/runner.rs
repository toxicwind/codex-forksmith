use std::path::Path;

use crate::config::{Config, ForkConfig};
use crate::engines;
use crate::process::{
    cargo_build_release, git_current_branch, git_divergence, git_fetch_remote, git_head_commit,
    git_is_clean, git_merge_abort, git_merge_ff_only, git_merge_with_strategy, git_reset_to_branch,
    git_stash_pop, git_stash_push,
};
use crate::registry::{PatchRegistry, PatchSet};
use anyhow::{anyhow, Result};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub dry_run: bool,
    pub skip_build: bool,
    pub emit_json: bool,
}

impl UpdateOptions {
    pub fn new(dry_run: bool, skip_build: bool, emit_json: bool) -> Self {
        Self {
            dry_run,
            skip_build,
            emit_json,
        }
    }
}

#[derive(Debug, Serialize)]
struct PatchReport {
    id: String,
    engine: String,
    status: String,
    matches: Option<u32>,
}

#[derive(Debug, Default, Serialize)]
pub struct UpdateSummary {
    dry_run: bool,
    vendor_head_before: Option<String>,
    vendor_head_after: Option<String>,
    patch_reports: Vec<PatchReport>,
    warnings: Vec<String>,
    build_status: Option<String>,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    workspace: String,
    vendor_dir: String,
    vendor_exists: bool,
    registry_path: String,
    registry_exists: bool,
    patch_sets_registered: usize,
}

pub fn run_health(root: &Path) -> Result<()> {
    let cfg = Config::load(root)?;
    let vendor = cfg.vendor_dir(root);
    let registry_path = cfg.registry_path(root);
    let registry = PatchRegistry::load_or_init(&cfg, root)?;

    let report = DoctorReport {
        workspace: root.display().to_string(),
        vendor_dir: vendor.display().to_string(),
        vendor_exists: vendor.exists(),
        registry_path: registry_path.display().to_string(),
        registry_exists: registry_path.exists(),
        patch_sets_registered: registry.list().len(),
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn run_list_patches(root: &Path) -> Result<()> {
    let cfg = Config::load(root)?;
    let registry = PatchRegistry::load_or_init(&cfg, root)?;
    for patch in registry.list() {
        println!(
            "- {:<32} engine={:?} enabled={} tags={}",
            patch.id,
            patch.engine,
            patch.enabled,
            if patch.tags.is_empty() {
                "-".into()
            } else {
                patch.tags.join(", ")
            }
        );
    }
    Ok(())
}

pub fn run_explain_patch(root: &Path, id: &str) -> Result<()> {
    let cfg = Config::load(root)?;
    let registry = PatchRegistry::load_or_init(&cfg, root)?;
    if let Some(p) = registry.get(id) {
        println!("Patch-set: {}", p.id);
        println!("  description : {}", p.description);
        println!("  engine      : {:?}", p.engine);
        println!("  enabled     : {}", p.enabled);
        println!("  rules       :");
        for r in &p.rules {
            println!("    - {}", r);
        }
        if !p.tags.is_empty() {
            println!("  tags        : {}", p.tags.join(", "));
        }
        if let Some(conf) = p.engine_confidence {
            println!("  confidence  : {:.2}", conf);
        }
        if let Some(status) = &p.last_status {
            println!("  last_status : {}", status);
        }
        if let Some(commit) = &p.last_applied_commit {
            println!("  last_commit : {}", commit);
        }
        if let Some(ts) = &p.last_run_ts {
            println!("  last_run_ts : {}", ts);
        }
    } else {
        anyhow::bail!("No patch-set with id {id}");
    }
    Ok(())
}

pub fn run_toggle_patch(root: &Path, id: &str, enabled: bool) -> Result<()> {
    let cfg = Config::load(root)?;
    let mut registry = PatchRegistry::load_or_init(&cfg, root)?;
    let patch = registry
        .get_mut(id)
        .ok_or_else(|| anyhow!("No patch-set with id {id}"))?;
    patch.enabled = enabled;
    registry.save(&cfg, root)?;
    println!("{} {}", if enabled { "Enabled" } else { "Disabled" }, id);
    Ok(())
}

pub fn run_update(root: &Path, opts: UpdateOptions) -> Result<()> {
    let cfg = Config::load(root)?;
    let vendor_dir = cfg.vendor_dir(root);
    if !vendor_dir.exists() {
        return Err(anyhow!(
            "Vendor directory {} does not exist",
            vendor_dir.display()
        ));
    }

    let mut summary = UpdateSummary {
        dry_run: opts.dry_run,
        vendor_head_before: git_head_commit(&vendor_dir).ok(),
        ..Default::default()
    };

    println!("codex-forksmith update");
    println!("  workspace root: {}", root.display());
    println!("  vendor dir    : {}", vendor_dir.display());
    println!("  dry-run       : {}", opts.dry_run);

    if cfg.fork.enabled {
        println!(
            "Step 1/4: Fork sync checks (local {} -> {}, upstream {} -> {})...",
            cfg.fork.local_remote,
            cfg.fork.local_branch,
            cfg.fork.upstream_remote,
            cfg.fork.upstream_branch
        );
        let mut fork_warnings = ensure_fork_state(&cfg, &vendor_dir)?;
        summary.warnings.append(&mut fork_warnings);
    } else {
        println!("Step 1/4: Reset vendor to origin/{}...", cfg.vendor_branch);
        git_reset_to_branch(&vendor_dir, &cfg.vendor_branch)?;
    }
    let commit = git_head_commit(&vendor_dir)?;
    summary.vendor_head_after = Some(commit.clone());

    println!("Step 2/4: Loading registry...");
    let mut registry = PatchRegistry::load_or_init(&cfg, root)?;
    println!("  {} patch-sets registered", registry.patch_sets.len());

    println!("Step 3/4: Applying patch-sets...");
    for patch in registry.patch_sets.clone() {
        if !patch.enabled {
            record_patch(&mut summary, &patch, None, "skipped (disabled)");
            continue;
        }
        let result = engines::apply_patchset(&patch, &cfg, &vendor_dir, opts.dry_run)?;
        record_patch(&mut summary, &patch, result.matches, result.status.clone());
        registry.update_after_run(&patch.id, &commit, result.matches, &result.status);
    }

    registry.save(&cfg, root)?;

    println!("Step 4/4: Build phase...");
    if opts.dry_run {
        summary.build_status = Some("skipped (dry-run)".into());
        println!("  build skipped (dry-run)");
    } else if opts.skip_build {
        summary.build_status = Some("skipped (--skip-build)".into());
        println!("  build skipped (--skip-build)");
    } else {
        cargo_build_release(&vendor_dir)?;
        summary.build_status = Some("succeeded".into());
        println!("  build succeeded");
    }

    if opts.emit_json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_summary(&summary);
    }

    Ok(())
}

fn record_patch(
    summary: &mut UpdateSummary,
    patch: &PatchSet,
    matches: Option<u32>,
    status: impl Into<String>,
) {
    summary.patch_reports.push(PatchReport {
        id: patch.id.clone(),
        engine: format!("{:?}", patch.engine),
        status: status.into(),
        matches,
    });
}

fn print_summary(summary: &UpdateSummary) {
    println!("\nSummary:");
    println!("  vendor before : {:?}", summary.vendor_head_before);
    println!("  vendor after  : {:?}", summary.vendor_head_after);
    println!("  dry-run       : {}", summary.dry_run);
    if !summary.patch_reports.is_empty() {
        println!("  patches:");
        for report in &summary.patch_reports {
            println!(
                "    - {:<32} {:<12} matches={:?} status={}",
                report.id, report.engine, report.matches, report.status
            );
        }
    }
    if !summary.warnings.is_empty() {
        println!("  warnings:");
        for w in &summary.warnings {
            println!("    - {w}");
        }
    }
    println!("  build        : {:?}", summary.build_status);
}

fn auto_merge_reference(
    vendor_dir: &Path,
    target_ref: &str,
    behind: u32,
    fork_cfg: &ForkConfig,
    allow_route_on_ff_failure: bool,
    label: &str,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let mut stashed = false;
    if !git_is_clean(vendor_dir)? {
        if fork_cfg.auto_stash_before_merge {
            stashed = git_stash_push(vendor_dir, true, "codex-forksmith auto-merge backup")?;
        } else if fork_cfg.abort_on_divergence {
            return Err(anyhow!(
                "{label} has {behind} commit(s) you still need to merge, but vendor/codex has local modifications. Commit or stash them, or enable fork.auto_stash_before_merge."
            ));
        } else {
            warnings.push(format!(
                "{label} is ahead by {behind} commit(s) but vendor/codex is dirty; merge it manually."
            ));
            return Ok(());
        }
    }

    let merge_status = match git_merge_ff_only(vendor_dir, target_ref) {
        Ok(_) => {
            warnings.push(format!("Fast-forwarded to {label} ({behind} commit(s))."));
            Ok(())
        }
        Err(ff_err) => {
            if allow_route_on_ff_failure {
                warnings.push(format!(
                    "Skipping {label} because fast-forward failed ({ff_err}). Leaving vendor/codex on its previous commit so the build can continue."
                ));
                if stashed {
                    if let Err(pop_err) = git_stash_pop(vendor_dir) {
                        warnings.push(format!(
                            "Skipped {label} but failed to reapply stashed changes: {pop_err}. Run `git stash pop --index` manually."
                        ));
                    }
                }
                return Ok(());
            }

            let result = git_merge_with_strategy(
                vendor_dir,
                target_ref,
                fork_cfg.merge_strategy.as_deref(),
                fork_cfg.merge_strategy_option.as_deref(),
            );
            match result {
                Ok(_) => {
                    let mut note = format!("Merged {label}");
                    if let Some(strategy) = &fork_cfg.merge_strategy {
                        note.push_str(&format!(" using -s {strategy}"));
                    } else {
                        note.push_str(" using git's default strategy");
                    }
                    if let Some(opt) = &fork_cfg.merge_strategy_option {
                        note.push_str(&format!(" (-X {opt})"));
                    }
                    note.push_str(&format!(" after --ff-only failed: {ff_err}"));
                    warnings.push(note);
                    Ok(())
                }
                Err(fallback_err) => {
                    let _ = git_merge_abort(vendor_dir);
                    Err(anyhow!(
                        "Auto-merge fallback failed: {fallback_err} (fast-forward error: {ff_err})."
                    ))
                }
            }
        }
    };

    if stashed {
        if let Err(pop_err) = git_stash_pop(vendor_dir) {
            warnings.push(format!(
                "Auto-merge completed but reapplying stashed changes failed: {pop_err}. Run `git stash pop --index` manually."
            ));
        }
    }

    merge_status
}

fn ensure_fork_state(cfg: &Config, vendor_dir: &Path) -> Result<Vec<String>> {
    let fork_cfg = &cfg.fork;
    let mut warnings = Vec::new();

    let branch = git_current_branch(vendor_dir)?;
    if branch != fork_cfg.local_branch {
        return Err(anyhow!(
            "Fork mode requires branch {} but the repo is currently on {}. Checkout {} before running the updater.",
            fork_cfg.local_branch,
            branch,
            fork_cfg.local_branch
        ));
    }

    if fork_cfg.require_clean_worktree && !git_is_clean(vendor_dir)? {
        return Err(anyhow!(
            "Vendor repo has local modifications. Commit, stash, or clean the tree before running the updater in fork mode."
        ));
    }

    git_fetch_remote(vendor_dir, &fork_cfg.local_remote)?;
    let needs_upstream_fetch = fork_cfg.upstream_remote != fork_cfg.local_remote
        || fork_cfg.upstream_branch != fork_cfg.local_branch;
    if needs_upstream_fetch {
        git_fetch_remote(vendor_dir, &fork_cfg.upstream_remote)?;
    }

    let tracking_ref = format!("{}/{}", fork_cfg.local_remote, fork_cfg.local_branch);
    match git_divergence(vendor_dir, "HEAD", &tracking_ref) {
        Ok((ahead, behind)) => {
            if behind > 0 {
                if fork_cfg.auto_merge_local {
                    auto_merge_reference(
                        vendor_dir,
                        &tracking_ref,
                        behind,
                        fork_cfg,
                        false,
                        &tracking_ref,
                        &mut warnings,
                    )?;
                } else {
                    let msg = format!(
                        "{} is ahead by {behind} commit(s). Pull or merge `{}` before running the updater.",
                        tracking_ref, tracking_ref
                    );
                    if fork_cfg.abort_on_divergence {
                        return Err(anyhow!(msg));
                    } else {
                        warnings.push(msg);
                    }
                }
            }
            if ahead > 0 {
                warnings.push(format!(
                    "Local branch is ahead of {tracking_ref} by {ahead} commit(s); remember to push after the run."
                ));
            }
        }
        Err(err) => warnings.push(format!(
            "Unable to compute divergence against {tracking_ref}: {err}"
        )),
    }

    let upstream_ref = format!("{}/{}", fork_cfg.upstream_remote, fork_cfg.upstream_branch);
    match git_divergence(vendor_dir, "HEAD", &upstream_ref) {
        Ok((ahead, behind)) => {
            if ahead > 0 && !fork_cfg.silence_local_ahead_warning {
                warnings.push(format!(
                    "Local branch carries {ahead} commit(s) not yet in {upstream_ref}."
                ));
            }
            if behind > 0 {
                if fork_cfg.auto_merge_upstream {
                    auto_merge_reference(
                        vendor_dir,
                        &upstream_ref,
                        behind,
                        fork_cfg,
                        fork_cfg.auto_route_upstream,
                        &upstream_ref,
                        &mut warnings,
                    )?;
                } else if fork_cfg.abort_on_divergence {
                    return Err(anyhow!(
                    "{upstream_ref} has {behind} commit(s) you still need to merge. Run `git merge {upstream_ref}` first or enable auto_merge_upstream."
                    ));
                } else {
                    warnings.push(format!(
                        "{upstream_ref} is ahead by {behind} commit(s); merge it before pushing."
                    ));
                }
            }
        }
        Err(err) => warnings.push(format!(
            "Unable to compute divergence against {upstream_ref}: {err}"
        )),
    }

    Ok(warnings)
}
