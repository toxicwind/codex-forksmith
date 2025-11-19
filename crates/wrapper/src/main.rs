use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use chrono::Utc;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

fn main() -> Result<()> {
    init_tracing();
    let config = WrapperConfig::from_env()?;
    maybe_run_update(&config)?;
    exec_codex(&config)
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

#[derive(Debug, Clone)]
struct WrapperConfig {
    updater_bin: Utf8PathBuf,
    workspace_root: Utf8PathBuf,
    codex_bin: Utf8PathBuf,
    stamp_file: PathBuf,
    auto_interval: Duration,
}

impl WrapperConfig {
    fn from_env() -> Result<Self> {
        let home = env::var("HOME").context("HOME unset")?;
        let workspace = env::var("CODEX_WORKSPACE")
            .or_else(|_| env::var("CODEX_FORKSMITH_WORKSPACE"))
            .or_else(|_| env::var("CODEX_PATCHER_WORKSPACE"))
            .or_else(|_| env::var("CODEX_PATCHER_UPDATER_WORKSPACE"))
            .unwrap_or_else(|_| {
                let new_path = format!("{home}/development/codex-forksmith");
                let legacy_path = format!("{home}/development/codex-patcher-updater");
                if std::path::Path::new(&new_path).exists()
                    || !std::path::Path::new(&legacy_path).exists()
                {
                    new_path
                } else {
                    legacy_path
                }
            });
        let updater_bin = env::var("CODEX_FORKSMITH")
            .or_else(|_| env::var("CODEX_PATCHER_UPDATER"))
            .or_else(|_| env::var("CODEX_UPDATER"))
            .unwrap_or_else(|_| {
                let fork_candidate = format!("{workspace}/target/debug/codex-forksmith");
                if std::path::Path::new(&fork_candidate).exists() {
                    fork_candidate
                } else {
                    format!("{workspace}/target/debug/codex-updater-cli")
                }
            });
        let codex_bin = env::var("CODEX_BIN")
            .unwrap_or_else(|_| format!("{workspace}/vendor/codex/target/debug/codex"));
        let stamp_dir = Utf8PathBuf::from(format!("{home}/.local/share/codex-wrapper"));
        fs::create_dir_all(&stamp_dir)?;
        let interval_secs: u64 = env::var("CODEX_WRAPPER_AUTO_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24 * 3600);
        Ok(Self {
            updater_bin: Utf8PathBuf::from(updater_bin),
            workspace_root: Utf8PathBuf::from(workspace),
            codex_bin: Utf8PathBuf::from(codex_bin),
            stamp_file: stamp_dir.join("last-update").into_std_path_buf(),
            auto_interval: Duration::from_secs(interval_secs),
        })
    }
}

fn maybe_run_update(config: &WrapperConfig) -> Result<()> {
    let needs_update = match fs::metadata(&config.stamp_file) {
        Ok(meta) => {
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            modified.elapsed().unwrap_or_default() > config.auto_interval
        }
        Err(_) => true,
    };
    if !needs_update {
        return Ok(());
    }
    info!(
        "running codex-forksmith for workspace {}",
        config.workspace_root
    );
    let status = Command::new(&config.updater_bin)
        .arg("update")
        .arg("--workspace")
        .arg(&config.workspace_root)
        .arg("--json")
        .status()
        .with_context(|| format!("launching {}", config.updater_bin))?;
    if status.success() {
        let now = Utc::now();
        fs::write(&config.stamp_file, now.to_rfc3339())?;
    } else {
        warn!("updater exited with {status}");
    }
    Ok(())
}

fn exec_codex(config: &WrapperConfig) -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let mut cmd = Command::new(&config.codex_bin);
    if args.is_empty() {
        args.push("--help".into());
    }
    let status = cmd
        .args(&args)
        .status()
        .with_context(|| format!("launching codex binary at {}", config.codex_bin))?;
    if !status.success() {
        anyhow::bail!("codex exited with {status}");
    }
    Ok(())
}
