# TODO

- Replace liteLLM config with local vLLM gateway (gpt2 currently) and fix uvicorn/uvloop import issues; run liteLLM proxy on port 30000 with config `litellm-gpt2.yaml`.
- Keep vllm-openai container (gpt2) running and validate `/v1/models` and chat completion; ensure persistence via tmux session `vllm-gateway`.
- Verify codex config (`~/.codex/config.toml`) MCP servers point to codex-rmcp-proxy once it is running; start codex-rmcp-proxy via its docker-compose or uvicorn entrypoint.
- Confirm remote-stack dev services remain healthy: backend `http://localhost:33180/api/health`, gateway `http://localhost:3000/api/health`, frontend `http://localhost:54321`.
- Clean up untracked `scripts/backend-import-smoke.sh` (decide whether to keep or delete) and commit pending tooling fixes (hb-tmux/hb-playwright edits) if desired.
