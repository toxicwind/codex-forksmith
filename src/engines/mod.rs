use std::path::Path;

use anyhow::Result;

use crate::config::Config;
use crate::registry::PatchSet;

pub struct EngineResult {
    pub matches: Option<u32>,
    pub status: String,
}

pub fn apply_patchset(
    patch: &PatchSet,
    cfg: &Config,
    vendor_dir: &Path,
    dry_run: bool,
) -> Result<EngineResult> {
    patch::apply(patch, cfg, vendor_dir, dry_run)
}

pub mod patch;
