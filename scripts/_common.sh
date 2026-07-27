#!/usr/bin/env bash
# Shared helpers for the scripts. Sourced, not executed.
set -euo pipefail

# Repo root (scripts live in scripts/).
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Load .env if present (export all vars).
load_env() {
  if [[ -f "$ROOT/.env" ]]; then
    set -a; # shellcheck disable=SC1091
    source "$ROOT/.env"; set +a
  fi
}

# Resolve the solpay binary: prefer PATH, then release, then debug build.
solpay_bin() {
  if command -v solpay >/dev/null 2>&1; then echo "solpay"; return; fi
  if [[ -x "$ROOT/target/release/solpay" ]]; then echo "$ROOT/target/release/solpay"; return; fi
  if [[ -x "$ROOT/target/debug/solpay" ]]; then echo "$ROOT/target/debug/solpay"; return; fi
  echo "" # not found
}

die()  { echo "error: $*" >&2; exit 1; }
info() { echo "  $*"; }
ok()   { echo "  ✓ $*"; }
