use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use camino::Utf8Path;

pub fn apply_legacy_patches(vendor_dir: &Utf8Path) -> Result<()> {
    for (name, patch) in LEGACY_PATCHES {
        apply_patch(name, patch, vendor_dir)?;
    }
    Ok(())
}

fn apply_patch(name: &str, patch: &str, vendor_dir: &Utf8Path) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("apply")
        .arg("--3way")
        .arg("--allow-empty")
        .arg("--whitespace=nowarn")
        .arg("-")
        .current_dir(vendor_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("spawning git apply for {name}"))?;
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin available");
        stdin.write_all(patch.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git apply failed for {name}: {stderr}");
    }
    Ok(())
}

const LEGACY_PATCHES: &[(&str, &str)] = &[
    ("fix-chatgpt-codex-mini-fallback", PATCH_CHATGPT_CODEX_MINI_FALLBACK),
    ("unbounded-exec-output", PATCH_UNBOUNDED_EXEC_OUTPUT),
    ("hb-unified-exec-trace", PATCH_HB_UNIFIED_EXEC_TRACE),
    ("hb-event-log", PATCH_HB_EVENT_LOG),
    ("allow-sudo-by-default", PATCH_ALLOW_SUDO_BY_DEFAULT),
];

const PATCH_CHATGPT_CODEX_MINI_FALLBACK: &str = r#"PLACEHOLDER"#;
const PATCH_UNBOUNDED_EXEC_OUTPUT: &str = r#"PLACEHOLDER"#;
const PATCH_HB_UNIFIED_EXEC_TRACE: &str = r#"PLACEHOLDER"#;
const PATCH_HB_EVENT_LOG: &str = r#"PLACEHOLDER"#;
const PATCH_ALLOW_SUDO_BY_DEFAULT: &str = r#"PLACEHOLDER"#;
