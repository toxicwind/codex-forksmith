use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    #[serde(alias = "ast_grep")]
    #[serde(alias = "coccinelle")]
    #[serde(alias = "gritql")]
    #[serde(alias = "patch")]
    Patch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSet {
    pub id: String,
    pub description: String,
    pub engine: EngineKind,
    pub enabled: bool,
    pub rules: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub engine_confidence: Option<f32>,
    #[serde(default)]
    pub last_applied_commit: Option<String>,
    #[serde(default)]
    pub last_match_count: Option<u32>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_run_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchRegistry {
    pub version: u32,
    pub generated_by: String,
    #[serde(default)]
    pub patch_sets: Vec<PatchSet>,
}

impl PatchRegistry {
    pub fn load_or_init(cfg: &Config, root: &Path) -> Result<Self> {
        let path = cfg.registry_path(root);
        if path.exists() {
            let data = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read registry at {}", path.display()))?;
            let reg: PatchRegistry =
                serde_json::from_str(&data).with_context(|| "Failed to parse registry JSON")?;
            Ok(reg)
        } else {
            Ok(PatchRegistry {
                version: 1,
                generated_by: "codex-forksmith 0.5.0".to_string(),
                patch_sets: Vec::new(),
            })
        }
    }

    pub fn save(&self, cfg: &Config, root: &Path) -> Result<()> {
        let path = cfg.registry_path(root);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create {}", dir.display()))?;
        }
        let data = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize registry JSON")?;
        fs::write(&path, data)
            .with_context(|| format!("Failed to write registry to {}", path.display()))?;
        Ok(())
    }

    pub fn list(&self) -> &[PatchSet] {
        &self.patch_sets
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut PatchSet> {
        self.patch_sets.iter_mut().find(|p| p.id == id)
    }

    pub fn get(&self, id: &str) -> Option<&PatchSet> {
        self.patch_sets.iter().find(|p| p.id == id)
    }

    pub fn update_after_run(
        &mut self,
        id: &str,
        commit: &str,
        match_count: Option<u32>,
        status: &str,
    ) {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        if let Some(patch) = self.get_mut(id) {
            let previous = patch.last_match_count;
            patch.last_applied_commit = Some(commit.to_string());
            patch.last_match_count = match_count;
            patch.last_run_ts = Some(now);

            let computed_status = if let Some(count) = match_count {
                if count == 0 {
                    if let Some(prev) = previous {
                        if prev > 0 {
                            format!("degraded: 0 matches (previously {prev})")
                        } else {
                            "no-matches".to_string()
                        }
                    } else {
                        "no-matches".to_string()
                    }
                } else {
                    format!("applied: {count} matches")
                }
            } else {
                status.to_string()
            };

            patch.last_status = Some(computed_status);
        }
    }
}
