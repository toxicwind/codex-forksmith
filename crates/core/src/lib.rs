use std::process::Command;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use codex_ast_driver::{AstGrepDriver, AstMode, AstRunOutcome};
use codex_cocci_driver::CocciDriver;
use codex_pkg::build_zip;
use codex_registry::{PatchResult, RegistryStore};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Serialize;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub workspace_root: Utf8PathBuf,
    pub vendor_dir: Utf8PathBuf,
    pub registry_path: Utf8PathBuf,
    pub ast_rules_dir: Option<Utf8PathBuf>,
    pub coccinelle_rules_dir: Option<Utf8PathBuf>,
    pub upstream_branch: String,
    pub cargo_check: bool,
    pub output_zip: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateSummary {
    pub vendor_rev_before: Option<String>,
    pub vendor_rev_after: Option<String>,
    pub ast_notes: Vec<String>,
    pub cocci_notes: Vec<String>,
    pub cargo_check_passed: bool,
    pub output_zip: Option<String>,
    pub warnings: Vec<String>,
}

pub fn run_update(opts: UpdateOptions) -> Result<UpdateSummary> {
    let mut summary = UpdateSummary {
        output_zip: opts.output_zip.as_ref().map(|p| p.to_string()),
        ..Default::default()
    };
    let vendor = opts.vendor_dir;
    let registry_store = RegistryStore::new(opts.registry_path.clone());
    let mut registry = registry_store.load()?;

    summary.vendor_rev_before = read_git_rev(&vendor).ok();
    sync_upstream(&vendor, &opts.upstream_branch)?;
    summary.vendor_rev_after = read_git_rev(&vendor).ok();

    let m = MultiProgress::new();
    let ast_pb = m.add(progress_spinner("ast-grep"));
    let cocci_pb = m.add(progress_spinner("coccinelle"));
    let cargo_pb = m.add(progress_spinner("cargo"));

    if let Some(ast_dir) = &opts.ast_rules_dir {
        if let Some(driver) = AstGrepDriver::detect(ast_dir)? {
            ast_pb.set_message("ast-grep dry-run");
            for set in registry.patch_sets.clone() {
                if !set.enabled {
                    registry.record_run(
                        &set.id,
                        None,
                        PatchResult::Skipped {
                            reason: Some("disabled".into()),
                        },
                    )?;
                    continue;
                }
                for rule in &set.rules {
                    let config_path = ast_dir.join(rule);
                    match driver.run_with_config(&config_path, &vendor, AstMode::DryRun)? {
                        AstRunOutcome::Applied(summary_run) => {
                            let estimated = summary_run.stdout.lines().count() as u64;
                            ast_pb.set_message(format!("{} → {} matches", set.id, estimated));
                            match driver.run_with_config(&config_path, &vendor, AstMode::Apply)? {
                                AstRunOutcome::Applied(apply_summary) => {
                                    summary.ast_notes.push(format!(
                                        "rule {} changed {} bytes",
                                        rule,
                                        apply_summary.stdout.len()
                                    ));
                                    registry.record_run(
                                        &set.id,
                                        Some(estimated),
                                        PatchResult::Applied {
                                            changed_files: estimated,
                                        },
                                    )?;
                                }
                                AstRunOutcome::Skipped { reason } => {
                                    warn!("ast rule {} skipped: {}", rule, reason);
                                    summary.warnings.push(reason.clone());
                                    registry.record_run(
                                        &set.id,
                                        Some(estimated),
                                        PatchResult::Skipped {
                                            reason: Some(reason),
                                        },
                                    )?;
                                }
                            }
                        }
                        AstRunOutcome::Skipped { reason } => {
                            warn!("ast dry run {} skipped: {}", rule, reason);
                            registry.record_run(
                                &set.id,
                                None,
                                PatchResult::Skipped {
                                    reason: Some(reason),
                                },
                            )?;
                        }
                    }
                }
            }
        } else {
            summary
                .warnings
                .push("ast-grep binary not found; skipping".into());
        }
    }
    ast_pb.finish_with_message("ast-grep complete");

    if let Some(cocci_dir) = &opts.coccinelle_rules_dir {
        if let Some(driver) = CocciDriver::detect(cocci_dir)? {
            cocci_pb.set_message("coccinelle pass");
            let report = driver.run(&vendor)?;
            for item in &report.reports {
                let note = format!(
                    "{} -> success={} exit={:?}",
                    item.rule, item.success, item.exit_code
                );
                summary.cocci_notes.push(note);
            }
        } else {
            summary
                .warnings
                .push("coccinelle-for-rust missing; skipped".into());
        }
    }
    cocci_pb.finish_with_message("coccinelle complete");

    if opts.cargo_check {
        cargo_pb.set_message("cargo check");
        summary.cargo_check_passed = run_cargo_check(&vendor)?;
        cargo_pb.finish_with_message("cargo check complete");
    }

    if let Some(zip_path) = opts.output_zip.as_ref() {
        build_zip(&vendor, zip_path)?;
    }
    let _ = m.clear();

    registry_store.save(&registry)?;
    Ok(summary)
}

fn progress_spinner(label: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(label.to_string());
    pb
}

fn sync_upstream(vendor: &Utf8Path, branch: &str) -> Result<()> {
    run_cmd("git", &["fetch", "origin"], vendor)?;
    run_cmd(
        "git",
        &["reset", "--hard", &format!("origin/{branch}")],
        vendor,
    )?;
    Ok(())
}

fn read_git_rev(repo: &Utf8Path) -> Result<String> {
    let output = run_cmd("git", &["rev-parse", "HEAD"], repo)?;
    Ok(output.trim().to_string())
}

fn run_cargo_check(workdir: &Utf8Path) -> Result<bool> {
    run_cmd("cargo", &["check"], workdir).map(|_| true)
}

fn run_cmd(bin: &str, args: &[&str], dir: &Utf8Path) -> Result<String> {
    let output = Command::new(bin)
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("running {bin} in {dir}"))?;
    if !output.status.success() {
        anyhow::bail!("{bin} failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into())
}
