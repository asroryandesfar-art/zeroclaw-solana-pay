#!/usr/bin/env bash
# Start the ZeroClaw agent for the payment assistant.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"
load_env

if ! command -v zeroclaw >/dev/null 2>&1; then
  die "zeroclaw not found on PATH. Install ZeroClaw, run scripts/setup.sh, then re-run."
fi

echo "==> Preflight"
"$ROOT/scripts/verify_env.sh"

echo "==> Health checks"
zeroclaw doctor
zeroclaw channel doctor

echo "==> Starting agent (Ctrl-C to stop)"
# The gateway hosts the WhatsApp webhook on 127.0.0.1:42617; expose via a tunnel
# if using WhatsApp Cloud API. See docs/OPERATIONS.md.
exec zeroclaw run
