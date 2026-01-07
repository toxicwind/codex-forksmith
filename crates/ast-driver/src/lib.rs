use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use tracing::warn;
use which::which;

#[derive(Debug, Clone)]
pub struct AstGrepDriver {
    binary: Utf8PathBuf,
    rules_dir: Utf8PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum AstMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone)]
pub struct AstRunSummary {
    pub mode: AstMode,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
}

#[derive(Debug, Clone)]
pub enum AstRunOutcome {
    Applied(AstRunSummary),
    Skipped { reason: String },
}

impl AstGrepDriver {
    pub fn detect(config_dir: &Utf8Path) -> Result<Option<Self>> {
        if !config_dir.exists() {
            return Ok(None);
        }
        match which("ast-grep") {
            Ok(path) => {
                let binary = Utf8PathBuf::from_path_buf(path)
                    .unwrap_or_else(|p| Utf8PathBuf::from(p.to_string_lossy().to_string()));
                Ok(Some(Self {
                    binary,
                    rules_dir: config_dir.to_path_buf(),
                }))
            }
            Err(_) => Ok(None),
        }
    }

    pub fn with_binary(binary: impl Into<Utf8PathBuf>, rules_dir: impl Into<Utf8PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            rules_dir: rules_dir.into(),
        }
    }

    pub fn run(&self, target: &Utf8Path, mode: AstMode) -> Result<AstRunOutcome> {
        self.run_with_config(&self.rules_dir, target, mode)
    }

    pub fn run_with_config(
        &self,
        config_path: &Utf8Path,
        target: &Utf8Path,
        mode: AstMode,
    ) -> Result<AstRunOutcome> {
        if !config_path.exists() {
            return Ok(AstRunOutcome::Skipped {
                reason: format!("rule config {} missing", config_path),
            });
        }
        if !target.exists() {
            return Ok(AstRunOutcome::Skipped {
                reason: format!("target {} missing", target),
            });
        }

        let mut cmd = Command::new(&self.binary);
        cmd.arg("run")
            .arg("--config")
            .arg(config_path)
            .arg("--json")
            .arg(target)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match mode {
            AstMode::DryRun => {
                cmd.arg("--dry-run");
            }
            AstMode::Apply => {}
        }

        let start = Instant::now();
        let output = cmd
            .output()
            .with_context(|| format!("running ast-grep via {}", self.binary))?;
        let duration_ms = start.elapsed().as_millis();

        if !output.status.success() {
            warn!(
                "ast-grep exited with {}; stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(AstRunOutcome::Skipped {
                reason: format!("ast-grep exit {}", output.status),
            });
        }

        Ok(AstRunOutcome::Applied(AstRunSummary {
            mode,
            stdout: String::from_utf8_lossy(&output.stdout).into(),
            stderr: String::from_utf8_lossy(&output.stderr).into(),
            duration_ms,
        }))
    }
}
