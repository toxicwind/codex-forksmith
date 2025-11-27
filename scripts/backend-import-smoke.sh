#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

VENV="${VENV:-.venv}"
PYTHON="${PYTHON:-$VENV/bin/python}"

if [[ ! -x "$PYTHON" ]]; then
  echo "Creating venv at $VENV"
  python -m venv "$VENV"
fi

source "$VENV/bin/activate"
python -m pip install --upgrade pip >/dev/null
python -m pip install fastapi >/dev/null

export PYTHONPATH="$REPO_ROOT/apps/backend"

echo "Running FastAPI import smoke..."
"$PYTHON" - <<'PY'
import importlib
importlib.import_module("fastapi")
print("fastapi import ok")
PY
