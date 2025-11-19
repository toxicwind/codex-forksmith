use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WorkspaceSection {
    root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoSection {
    path: Option<String>,
    local_remote: Option<String>,
    local_branch: Option<String>,
    upstream_remote: Option<String>,
    upstream_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BuildSection {
    profile: Option<String>,
    workspace: Option<String>,
    binary_relpath: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    workspace: Option<WorkspaceSection>,
    repo: Option<RepoSection>,
    build: Option<BuildSection>,
}

#[derive(Debug, Clone)]
pub struct ForksmithConfig {
    pub workspace_root: PathBuf,
    pub repo_path: PathBuf,
    pub local_remote: String,
    pub local_branch: String,
    pub upstream_remote: String,
    pub upstream_branch: String,
    pub build_profile: String,
    pub build_workspace: PathBuf,
    pub binary_relpath: PathBuf,
}

impl ForksmithConfig {
    pub fn load_default() -> Result<Self> {
        Self::load_from_path("codex-forksmith.toml")
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let raw: RawConfig =
            toml::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;
        let config_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let workspace_root = resolve_path(
            &config_dir,
            raw.workspace
                .as_ref()
                .and_then(|w| w.root.clone())
                .unwrap_or_else(|| ".".to_string()),
        );

        let repo_section = raw.repo.unwrap_or_default();
        let repo_path = resolve_path(
            &workspace_root,
            repo_section
                .path
                .unwrap_or_else(|| "vendor/codex".to_string()),
        );

        let build_section = raw.build.unwrap_or_default();
        let build_workspace = resolve_path(
            &repo_path,
            build_section
                .workspace
                .unwrap_or_else(|| "codex-rs".to_string()),
        );
        let binary_relpath = PathBuf::from(
            build_section
                .binary_relpath
                .unwrap_or_else(|| "codex-rs/target/release/codex".to_string()),
        );

        Ok(Self {
            workspace_root,
            repo_path,
            local_remote: repo_section
                .local_remote
                .unwrap_or_else(|| "origin".to_string()),
            local_branch: repo_section
                .local_branch
                .unwrap_or_else(|| "main".to_string()),
            upstream_remote: repo_section
                .upstream_remote
                .unwrap_or_else(|| "upstream".to_string()),
            upstream_branch: repo_section
                .upstream_branch
                .unwrap_or_else(|| "main".to_string()),
            build_profile: build_section
                .profile
                .unwrap_or_else(|| "release".to_string()),
            build_workspace,
            binary_relpath,
        })
    }

    pub fn repo_binary_path(&self) -> PathBuf {
        self.repo_path.join(&self.binary_relpath)
    }
}

fn resolve_path(base: &Path, value: impl Into<PathBuf>) -> PathBuf {
    let candidate = value.into();
    if candidate.is_absolute() {
        candidate
    } else {
        base.join(candidate)
    }
}

impl Default for RepoSection {
    fn default() -> Self {
        Self {
            path: None,
            local_remote: None,
            local_branch: None,
            upstream_remote: None,
            upstream_branch: None,
        }
    }
}

impl Default for BuildSection {
    fn default() -> Self {
        Self {
            profile: None,
            workspace: None,
            binary_relpath: None,
        }
    }
}
