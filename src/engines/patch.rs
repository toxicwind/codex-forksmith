use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::engines::EngineResult;
use crate::registry::PatchSet;

pub fn apply(
    patch: &PatchSet,
    _cfg: &Config,
    vendor_dir: &Path,
    dry_run: bool,
) -> Result<EngineResult> {
    let workspace_root = vendor_dir
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(vendor_dir);

    let mut applied = 0u32;

    for rule in &patch.rules {
        let path = PathBuf::from(rule);
        let patch_path = if path.is_absolute() {
            path
        } else {
            workspace_root.join(path)
        };
        let data = fs::read(&patch_path)
            .with_context(|| format!("failed to read patch {}", patch_path.display()))?;

        let mut cmd = Command::new("git");
        cmd.arg("apply")
            .arg("--3way")
            .arg("--allow-empty")
            .arg("--whitespace=nowarn")
            .current_dir(vendor_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if dry_run {
            cmd.arg("--check");
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawning git apply for {}", patch_path.display()))?;
        {
            let stdin = child
                .stdin
                .as_mut()
                .context("patch runner failed to open stdin")?;
            stdin.write_all(&data)?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "git apply failed for {}: {}",
                patch_path.display(),
                stderr.trim()
            );
        }
        applied += 1;
    }

    Ok(EngineResult {
        matches: Some(applied),
        status: if dry_run {
            "dry-run".to_string()
        } else {
            "applied".to_string()
        },
    })
}
