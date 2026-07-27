#!/usr/bin/env bash
# Build + install solpay, bootstrap config, and (if ZeroClaw is present) link the
# agent config and skills into place.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

echo "==> Building and installing solpay"
( cd "$ROOT" && cargo install --path crates/solpay --locked )
ok "solpay installed to ~/.cargo/bin"

echo "==> Bootstrapping .env"
if [[ ! -f "$ROOT/.env" ]]; then
  cp "$ROOT/.env.example" "$ROOT/.env"
  chmod 600 "$ROOT/.env"
  info "created .env from .env.example — edit it and set MERCHANT_WALLET"
else
  ok ".env already exists"
fi

echo "==> Linking ZeroClaw config + skills"
if command -v zeroclaw >/dev/null 2>&1; then
  mkdir -p "$HOME/.zeroclaw" "$HOME/.zeroclaw/workspace/skills"
  ln -sf "$ROOT/agent/zeroclaw.toml" "$HOME/.zeroclaw/config.toml"
  ok "linked ~/.zeroclaw/config.toml -> agent/zeroclaw.toml"
  for d in "$ROOT"/agent/skills/*/; do
    name="$(basename "$d")"
    ln -sfn "$d" "$HOME/.zeroclaw/workspace/skills/$name"
    ok "linked skill: $name"
  done
  info "next: zeroclaw doctor && zeroclaw channel doctor"
else
  info "zeroclaw not found on PATH — install it, then re-run to link config/skills."
  info "(the solpay engine is fully usable now; see scripts/demo.sh)"
fi

echo "==> Done. Verify your environment with: scripts/verify_env.sh"
