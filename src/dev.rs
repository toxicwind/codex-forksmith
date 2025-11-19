use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use pathdiff::diff_paths;

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(400);
const IGNORE_PREFIXES: &[&str] = &[".git", "target"];
const IGNORE_VENDOR: &[&str] = &["vendor/codex"];

pub fn run_watch(root: &Path) -> Result<()> {
    println!("▶ starting auto fmt + lint watcher in {}", root.display());
    run_fmt_and_clippy(root)?;

    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default())
        .with_context(|| "Failed to initialize filesystem watcher")?;
    watcher
        .watch(root, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch {}", root.display()))?;

    event_loop(root.to_path_buf(), rx)
}

fn event_loop(root: PathBuf, rx: Receiver<Result<Event, notify::Error>>) -> Result<()> {
    let mut last_run = Instant::now();
    loop {
        let event = rx.recv().with_context(|| "Watcher channel disconnected")?;
        let event = match event {
            Ok(ev) => ev,
            Err(err) => {
                eprintln!("[watch] notify error: {err}");
                continue;
            }
        };

        if !should_trigger(&root, &event) {
            continue;
        }

        if last_run.elapsed() < DEBOUNCE_WINDOW {
            continue;
        }
        last_run = Instant::now();
        if let Err(err) = run_fmt_and_clippy(&root) {
            eprintln!("[watch] lint run failed: {err}");
        }
    }
}

fn should_trigger(root: &Path, event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    ) && event.paths.iter().any(|path| !is_ignored_path(root, path))
}

fn is_ignored_path(root: &Path, path: &Path) -> bool {
    if path.is_dir() {
        return true;
    }
    let rel = diff_paths(path, root).unwrap_or_else(|| path.to_path_buf());
    let mut comps = rel.components();
    if let Some(Component::Normal(first)) = comps.next() {
        if IGNORE_PREFIXES
            .iter()
            .any(|prefix| first.eq_ignore_ascii_case(prefix))
        {
            return true;
        }
        if first == "vendor" {
            if let Some(Component::Normal(second)) = comps.next() {
                let candidate = format!("vendor/{}", second.to_string_lossy());
                if IGNORE_VENDOR.iter().any(|entry| entry == &candidate) {
                    return true;
                }
            }
        }
    }
    false
}

fn run_fmt_and_clippy(root: &Path) -> Result<()> {
    println!("▶ running cargo fmt");
    run_cargo(&["fmt"], root)?;
    println!("▶ running cargo clippy --all-targets --all-features -D warnings");
    run_cargo(
        &[
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        root,
    )
}

fn run_cargo(args: &[&str], root: &Path) -> Result<()> {
    let label = format!("cargo {}", args.join(" "));
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("Failed to spawn {label}"))?;
    if !status.success() {
        return Err(anyhow!("{label} failed with status {:?}", status.code()));
    }
    Ok(())
}
