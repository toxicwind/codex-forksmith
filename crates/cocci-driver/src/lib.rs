use std::fs;
use std::process::Command;

use anyhow::{Context, Result};
use camino::{FromPathBufError, Utf8Path, Utf8PathBuf};
use tracing::warn;
use which::which;

#[derive(Debug, Clone)]
pub struct CocciDriver {
    binary: Utf8PathBuf,
    rules_dir: Utf8PathBuf,
}

#[derive(Debug, Clone)]
pub struct CocciRuleReport {
    pub rule: Utf8PathBuf,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct CocciSummary {
    pub reports: Vec<CocciRuleReport>,
}

impl CocciDriver {
    fn fallback_utf8_path(err: FromPathBufError) -> Utf8PathBuf {
        let path = err.into_path_buf();
        Utf8PathBuf::from(path.to_string_lossy().to_string())
    }

    pub fn detect(rules_dir: &Utf8Path) -> Result<Option<Self>> {
        if !rules_dir.exists() {
            return Ok(None);
        }
        match which("coccinelle-for-rust") {
            Ok(path) => {
                let binary = Utf8PathBuf::try_from(path).unwrap_or_else(Self::fallback_utf8_path);
                Ok(Some(Self {
                    binary,
                    rules_dir: rules_dir.to_path_buf(),
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

    pub fn run(&self, target: &Utf8Path) -> Result<CocciSummary> {
        if !self.rules_dir.exists() {
            return Ok(CocciSummary { reports: vec![] });
        }
        let mut reports = Vec::new();
        for entry in
            fs::read_dir(&self.rules_dir).with_context(|| format!("reading {}", self.rules_dir))?
        {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path()).unwrap_or_else(Self::fallback_utf8_path);
            if path.extension() != Some("cocci") {
                continue;
            }
            let output = Command::new(&self.binary)
                .arg("--patch")
                .arg(&path)
                .arg(target)
                .output();
            match output {
                Ok(out) => {
                    reports.push(CocciRuleReport {
                        rule: path.clone(),
                        exit_code: out.status.code(),
                        stdout: String::from_utf8_lossy(&out.stdout).into(),
                        stderr: String::from_utf8_lossy(&out.stderr).into(),
                        success: out.status.success(),
                    });
                    if !out.status.success() {
                        warn!("coccinelle rule {} failed: {}", path, out.status);
                    }
                }
                Err(err) => {
                    reports.push(CocciRuleReport {
                        rule: path.clone(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: err.to_string(),
                        success: false,
                    });
                    warn!("failed to run coccinelle on {}: {err}", path);
                }
            }
        }
        Ok(CocciSummary { reports })
    }
}
