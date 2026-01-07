use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use fs_err as fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchSet {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_applied_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_match_count: Option<u64>,
    #[serde(default)]
    pub last_result: Option<PatchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PatchResult {
    Applied { changed_files: u64 },
    Skipped { reason: Option<String> },
    Failed { error: String },
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Registry {
    #[serde(default)]
    pub patch_sets: Vec<PatchSet>,
}

impl Registry {
    pub fn load(path: &Utf8Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).with_context(|| format!("reading registry {path}"))?;
        let registry: Registry = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing registry json {}", path))?;
        Ok(registry)
    }

    pub fn save(&self, path: &Utf8Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn toggle(&mut self, id: &str, enabled: bool) -> Result<()> {
        let set = self
            .patch_sets
            .iter_mut()
            .find(|p| p.id == id)
            .with_context(|| format!("patch set {id} not found"))?;
        set.enabled = enabled;
        Ok(())
    }

    pub fn record_run(
        &mut self,
        id: &str,
        match_count: Option<u64>,
        result: PatchResult,
    ) -> Result<()> {
        let now = Utc::now();
        let set = self
            .patch_sets
            .iter_mut()
            .find(|p| p.id == id)
            .with_context(|| format!("patch set {id} not found"))?;
        set.last_applied_at = Some(now);
        set.last_match_count = match_count;
        set.last_result = Some(result);
        Ok(())
    }

    pub fn ensure_patch_set<F>(&mut self, templ: PatchSetTemplate, build_notes: F) -> &PatchSet
    where
        F: FnOnce() -> Option<String>,
    {
        if let Some(idx) = self.patch_sets.iter().position(|p| p.id == templ.id) {
            return &self.patch_sets[idx];
        }
        let mut new_set = templ.into_patch_set();
        let inserted_id = new_set.id.clone();
        new_set.notes = build_notes();
        self.patch_sets.push(new_set);
        self.patch_sets.sort_by(|a, b| a.id.cmp(&b.id));
        let idx = self
            .patch_sets
            .iter()
            .position(|p| p.id == inserted_id)
            .expect("just inserted patch set");
        &self.patch_sets[idx]
    }
}

#[derive(Debug, Clone)]
pub struct PatchSetTemplate {
    pub id: String,
    pub description: String,
    pub rules: Vec<String>,
    pub tags: Vec<String>,
}

impl PatchSetTemplate {
    pub fn into_patch_set(self) -> PatchSet {
        PatchSet {
            id: self.id,
            description: self.description,
            rules: self.rules,
            enabled: true,
            tags: self.tags,
            notes: None,
            created_at: Some(Utc::now()),
            last_applied_at: None,
            last_match_count: None,
            last_result: None,
        }
    }
}

fn default_enabled() -> bool {
    true
}

pub struct RegistryStore {
    path: Utf8PathBuf,
}

impl RegistryStore {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Registry> {
        Registry::load(&self.path)
    }

    pub fn save(&self, registry: &Registry) -> Result<()> {
        registry.save(&self.path)
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }
}
