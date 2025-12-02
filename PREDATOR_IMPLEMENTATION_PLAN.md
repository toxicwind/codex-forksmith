# Predator Implementation Plan

1. **Tooling consolidation** – retire references to `hb-github-finder`, rename the Emergent Discovery Doctrine to `github-advanced-search-mcp`, and document the Linux-only wrapper as the canonical search helper (see `AGENTS.md`/`AGENTS.bak`).
2. **Project pivot** – keep the `development/hb-gh-search` repo as the build-root and expose it through `~/github-advanced-search-mcp` plus `/home/toxic/.config/bash/hypebrut/bin/github-advanced-search-mcp`, ensuring all scripts/platforms hit the same binary.
3. **Loader hardening** – continue tracing stdout/stderr via the inline Python filter, capture logs under `/tmp/antigravity-loader-logs/*`, and maintain `LD_PRELOAD`/`stdbuf` so CortexStepRunCommand sees clean JSON alongside merged stderr.
4. **Documentation follow-through** – audit docs (AGENTS, AGENTS.bak, `docs/` references) for legacy GH search commands, update them to the new wrapper, and record the consolidation so future agents avoid Windows-targeted tooling.
5. **Validation loop** – run smoke tests (language server commands, `KEEP_LOGS=1` flows) while `tail -F /tmp/antigravity-loader-logs/*.stderr` to prove noise stays on stderr, then sign off the plan with log evidence.
