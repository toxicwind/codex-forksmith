# Codex Forksmith

![codex-forksmith banner](https://img.shields.io/badge/Codex-Forksmith-blue?logo=rust)

An ergonomic, Rust-native control plane for the vendored Codex workspace.
Codex Forksmith provides a small wrapper (`codex`) that standardizes status,
sync, build, and run workflows so humans and automation (agents) can operate
reliably against `vendor/codex`.

---

## Quick links

- **CLI:** `codex` — wrapper that runs `cargo run --bin codex-forksmith` from this repo
- **Build:** `cargo build --workspace`
- **Tests:** `cargo test --workspace`
- **CI:** GitHub Actions under `.github/workflows/ci.yml`

---

## Table of contents

- [Codex Forksmith](#codex-forksmith)
  - [Quick links](#quick-links)
  - [Table of contents](#table-of-contents)
  - [Control plane (codex)](#control-plane-codex)
  - [Workflows](#workflows)
  - [**Control Plane**](#control-plane)
  - [**Workflows**](#workflows-1)
  - [**Workspace Layout**](#workspace-layout)
  - [**Configuration**](#configuration)
  - [**Maintainer Notes**](#maintainer-notes)

---

## Control plane (codex)

`codex` is a thin, agent-friendly control plane that exposes predictable
operations over the vendored Codex workspace. Running `codex` with no args
prints a short menu of common tasks and exits with status `0`. Full help is
available via `codex --help`.

Primary subcommands:

- `codex status`
  - Inspects repository state and `vendor/codex`:
    - current branch and HEAD
    - working tree cleanliness
    - ahead/behind counts vs `origin/<branch>` and `upstream/<branch>`
    - detects merge conflicts and missing artifact
  - Exits non‑zero only on merge conflicts or when the compiled binary is missing.

- `codex sync [--dry-run]`
  - Fetches configured remotes and applies fast-forwards when safe.
  - Idempotent and safe to run repeatedly. When complete it prints a single
    machine-readable summary line beginning with `SYNC_RESULT` for agent parsing.

- `codex build`
  - Runs the configured `cargo build` (by default release profile) in the
    vendored Codex workspace and prints the artifact path.
  - Warns if the repo is dirty but still builds.

- `codex run -- <args>`
  - Ensures the Codex binary exists (auto-runs `codex build` if missing) and
    then execs it, inheriting stdin/stdout/stderr for clean passthrough.

Use these commands in automation and agent workflows instead of invoking raw
`git`/`cargo`—they are conservative, machine-friendly, and clearly signal
outcomes.

---

## Workflows

Example human session:

```bash
codex status
codex sync --dry-run
codex build
codex run -- --help
```

Suggested verification locally:

```bash
cd ~/development/codex-forksmith
<!-- prettier, lightweight README for repository landing -->
# Codex Forksmith

![CI](https://github.com/toxicwind/codex-forksmith/actions/workflows/ci.yml/badge.svg)
![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-blue)

A focused, Rust-native control plane for the vendored `codex` workspace. Use
the lightweight `codex` wrapper to inspect status, sync remotes, build the
vendored binary, and exec it from automation or a human shell.

---

## Quick Start

- Build everything: `cargo build --workspace`
- Run tests: `cargo test --workspace`
- Run the control plane: `cargo run --bin codex-forksmith -- <subcommand>`

Common shortcuts (when `codex` is installed as a local CLI wrapper):

```bash
codex              # shows banner + useful workflow hints
codex status       # inspect workspace and binary health
codex sync --dry-run  # safe fetch + parseable SYNC_RESULT summary
codex build        # compile vendor/codex and print artifact path
codex run -- --help  # passthrough to compiled codex binary
```

---

## **Control Plane**

`codex` exposes a small, opinionated set of commands designed for reliability
and automation:

- **status**: reports branch, cleanliness, ahead/behind, merge conflicts,
  and whether the compiled binary exists. Exits non‑zero only on merge
  conflicts or when the compiled binary is missing.
- **sync [--dry-run]**: fetches remotes and fast-forwards when safe. Always
  prints a single machine-readable `SYNC_RESULT` line describing the outcome.
- **build**: runs `cargo build` (default: release) in the vendored workspace
  and prints the produced artifact path; warns on a dirty tree but still builds.
- **run -- <args>**: ensures the binary exists (auto-builds if missing), then
  `exec`s it, inheriting stdin/stdout/stderr so output chains cleanly.

Use these commands in CI or agent workflows to avoid brittle raw `git`/`cargo`
invocations.

---

## **Workflows**

Suggested local verification:

```bash
cd ~/development/codex-forksmith
cargo fmt && cargo test
codex
codex status
codex sync --dry-run
codex build
codex run -- --help
```

---

## **Workspace Layout**

Top-level crates live under `crates/`. Notable items:

- `crates/ast-driver`, `crates/cocci-driver` — adapters used by the update pipeline
- `crates/core` — orchestration primitives and core types
- `crates/registry` — JSON registry helpers for patch sets
- `crates/pkg` — packaging helpers
- `crates/wrapper` — small wrapper/launcher

To build everything: `cargo build --workspace`.

---

## **Configuration**

Edit `codex-forksmith.toml` to change repo and build settings. Example:

```toml
[repo]
path = "vendor/codex"
local_remote = "origin"
local_branch = "main"
upstream_remote = "upstream"
upstream_branch = "main"

[build]
profile = "release"
binary_relpath = "codex-rs/target/release/codex"
```

Defaults are sensible; only override what you need.

---

## **Maintainer Notes**

- `crates/updater-cli` and historical updater sources have been removed from
  the active workspace. Historical files were retained for reference and have
  now been cleaned up.
- CI is active via `.github/workflows/ci.yml`.

If you'd like additional badges or a different landing image, tell me which
services to include and I'll add them.
