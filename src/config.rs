use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Default)]
pub struct VendorSection {
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PatchRegistrySection {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ForkSection {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub upstream_remote: Option<String>,
    #[serde(default)]
    pub upstream_branch: Option<String>,
    #[serde(default)]
    pub local_remote: Option<String>,
    #[serde(default)]
    pub local_branch: Option<String>,
    #[serde(default)]
    pub require_clean_worktree: Option<bool>,
    #[serde(default)]
    pub abort_on_divergence: Option<bool>,
    #[serde(default)]
    pub auto_merge_upstream: Option<bool>,
    #[serde(default)]
    pub auto_stash_before_merge: Option<bool>,
    #[serde(default)]
    pub auto_merge_local: Option<bool>,
    #[serde(default)]
    pub auto_route_upstream: Option<bool>,
    #[serde(default)]
    pub merge_strategy: Option<String>,
    #[serde(default)]
    pub merge_strategy_option: Option<String>,
    #[serde(default)]
    pub silence_local_ahead_warning: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawConfig {
    #[serde(default)]
    pub vendor: VendorSection,
    #[serde(default)]
    pub patch_registry: PatchRegistrySection,
    #[serde(default)]
    pub fork: ForkSection,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub vendor_root: String,
    pub vendor_branch: String,
    pub patch_registry_path: String,
    pub fork: ForkConfig,
}

#[derive(Debug, Clone)]
pub struct ForkConfig {
    pub enabled: bool,
    pub upstream_remote: String,
    pub upstream_branch: String,
    pub local_remote: String,
    pub local_branch: String,
    pub require_clean_worktree: bool,
    pub abort_on_divergence: bool,
    pub auto_merge_upstream: bool,
    pub auto_stash_before_merge: bool,
    pub auto_merge_local: bool,
    pub auto_route_upstream: bool,
    pub merge_strategy: Option<String>,
    pub merge_strategy_option: Option<String>,
    pub silence_local_ahead_warning: bool,
}

impl Config {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("codex-forksmith.toml");
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let raw: RawConfig =
            toml::from_str(&contents).with_context(|| "Failed to parse codex-forksmith.toml")?;

        let vendor_root = raw
            .vendor
            .root
            .unwrap_or_else(|| "vendor/codex".to_string());
        let vendor_branch = raw.vendor.branch.unwrap_or_else(|| "main".to_string());

        let patch_registry_path = raw
            .patch_registry
            .path
            .unwrap_or_else(|| "patch-registry/registry.json".to_string());

        let fork = ForkConfig::from_section(&raw.fork, &vendor_branch);

        Ok(Config {
            vendor_root,
            vendor_branch,
            patch_registry_path,
            fork,
        })
    }

    pub fn vendor_dir(&self, root: &Path) -> PathBuf {
        root.join(&self.vendor_root)
    }

    pub fn registry_path(&self, root: &Path) -> PathBuf {
        root.join(&self.patch_registry_path)
    }
}

impl ForkConfig {
    fn from_section(section: &ForkSection, vendor_branch: &str) -> Self {
        Self {
            enabled: section.enabled.unwrap_or(false),
            upstream_remote: section
                .upstream_remote
                .clone()
                .unwrap_or_else(|| "upstream".to_string()),
            upstream_branch: section
                .upstream_branch
                .clone()
                .unwrap_or_else(|| vendor_branch.to_string()),
            local_remote: section
                .local_remote
                .clone()
                .unwrap_or_else(|| "origin".to_string()),
            local_branch: section
                .local_branch
                .clone()
                .unwrap_or_else(|| vendor_branch.to_string()),
            require_clean_worktree: section.require_clean_worktree.unwrap_or(true),
            abort_on_divergence: section.abort_on_divergence.unwrap_or(true),
            auto_merge_upstream: section.auto_merge_upstream.unwrap_or(false),
            auto_stash_before_merge: section.auto_stash_before_merge.unwrap_or(true),
            auto_merge_local: section.auto_merge_local.unwrap_or(false),
            auto_route_upstream: section.auto_route_upstream.unwrap_or(false),
            merge_strategy: section.merge_strategy.clone(),
            merge_strategy_option: section.merge_strategy_option.clone(),
            silence_local_ahead_warning: section.silence_local_ahead_warning.unwrap_or(false),
        }
    }
}
