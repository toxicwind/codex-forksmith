#!/usr/bin/env bash
set -euo pipefail

TS=$(date +%s)
echo "[cleanup] started at $(date -u)"

echo "[cleanup] backing up ~/.bashrc -> ~/.bashrc.pre_hb_backup.$TS"
cp --preserve=mode,timestamp ~/.bashrc ~/.bashrc.pre_hb_backup.$TS

echo "[cleanup] removing VS Code integration snippets from ~/.bashrc"
awk '!/code --locate-shell-integration-path/ && !/__vscode_shell_integration_prompt/ && !/VSCODE_SHELL_INTEGRATION/' ~/.bashrc > ~/.bashrc.cleanup.tmp
mv ~/.bashrc.cleanup.tmp ~/.bashrc
echo "[cleanup] backup saved; edits applied"

echo "[cleanup] ensuring globals.sh backup"
mkdir -p ~/.config/bash/hypebrut
if [[ -f ~/.config/bash/hypebrut/globals.sh ]]; then
  cp --preserve=mode,timestamp ~/.config/bash/hypebrut/globals.sh ~/.config/bash/hypebrut/globals.sh.bak.$TS
  echo "[cleanup] globals.sh backed up"
else
  echo "[cleanup] globals.sh not present (unexpected)"
fi

echo "[cleanup] cleaning stale target artifacts for codex-patcher-updater"
rm -f target/release/codex-patcher-updater target/release/codex-patcher-updater.d || true
find target/.fingerprint -maxdepth 1 -type d -name '*codex-patcher-updater*' -print0 | xargs -0r rm -rf || true

echo "[cleanup] if codex-patcher-updater crate exists in cargo metadata, cargo clean it"
if cargo metadata --no-deps --format-version=1 >/dev/null 2>&1; then
  if cargo metadata --no-deps --format-version=1 | grep -q 'codex-patcher-updater'; then
    echo "[cleanup] running cargo clean -p codex-patcher-updater"
    cargo clean -p codex-patcher-updater || true
  fi
fi

echo "[cleanup] ensure ~/.config/bash/hypebrut is a git repo and commit globals.sh"
HB_DIR="$HOME/.config/bash/hypebrut"
if [[ -d "$HB_DIR/.git" ]]; then
  echo "[cleanup] hypebrut already in git. committing changes"
  git -C "$HB_DIR" add globals.sh || true
  if git -C "$HB_DIR" diff --staged --quiet; then
    echo "[cleanup] no changes to commit in hypebrut"
  else
    git -C "$HB_DIR" commit -m "hypebrut: preload VS Code shell integration and preserve PROMPT_COMMAND" || true
  fi
else
  echo "[cleanup] initializing hypebrut git repo at $HB_DIR"
  git -C "$HB_DIR" init || true
  git -C "$HB_DIR" add globals.sh || true
  git -C "$HB_DIR" commit -m "initial commit: hypebrut globals (preload VS Code integration)" || true
  echo "[cleanup] created local git repo for hypebrut (no remote added)"
fi

echo "[cleanup] committing repo-level deletions (if any)"
git add -A || true
if git diff --staged --quiet; then
  echo "[cleanup] nothing to commit at repo root"
else
  git commit -m "chore: remove legacy patch-registry and related artifacts; clean workspace" || true
  echo "[cleanup] committed changes to repo"
fi

echo "[cleanup] done"
