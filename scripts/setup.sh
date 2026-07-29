#!/usr/bin/env bash
# Build + install solpay, then deploy the ZeroClaw agent layer into a config dir
# and validate it. Idempotent: safe to re-run.
#
# Validated against ZeroClaw v0.8.3 (schema_version 3).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

CONFIG_DIR="${ZEROCLAW_CONFIG_DIR:-$HOME/.zeroclaw}"
SOLPAY_ENV="$HOME/.zeroclaw/solpay.env"   # $HOME-relative: skills source this

echo "==> Building and installing solpay"
( cd "$ROOT" && cargo install --path crates/solpay --locked )
ok "solpay installed to ~/.cargo/bin"

echo "==> Bootstrapping the locked money-path config (~/.zeroclaw/solpay.env)"
mkdir -p "$HOME/.zeroclaw"
if [[ ! -f "$SOLPAY_ENV" ]]; then
  cp "$ROOT/agent/solpay.env.example" "$SOLPAY_ENV"
  chmod 600 "$SOLPAY_ENV"
  info "created $SOLPAY_ENV — edit it and set MERCHANT_WALLET (public receiving key)"
else
  ok "$SOLPAY_ENV already exists"
fi

if ! command -v zeroclaw >/dev/null 2>&1; then
  info "zeroclaw not found on PATH — install from https://github.com/zeroclaw-labs/zeroclaw"
  info "(the solpay engine is fully usable now; see scripts/demo.sh)"
  exit 0
fi

echo "==> Deploying agent layer into $CONFIG_DIR"
mkdir -p "$CONFIG_DIR"
cp "$ROOT/agent/config.toml" "$CONFIG_DIR/config.toml"
cp -r "$ROOT/agent/skills"  "$CONFIG_DIR/"
cp -r "$ROOT/agent/sops"    "$CONFIG_DIR/"
cp -r "$ROOT/agent/prompts" "$CONFIG_DIR/"
ok "copied config.toml, skills/, sops/, prompts/"

# sop.sops_dir must be absolute; set it via the real tool.
zeroclaw config set sop.sops_dir "$CONFIG_DIR/sops" --config-dir "$CONFIG_DIR" >/dev/null
ok "set sop.sops_dir -> $CONFIG_DIR/sops"

echo "==> Validating"
zeroclaw skills list  --config-dir "$CONFIG_DIR"
zeroclaw sop validate --config-dir "$CONFIG_DIR"
zeroclaw doctor       --config-dir "$CONFIG_DIR" | sed -n '1,20p'

cat <<EOF

Next steps (operator, one time):
  1. Edit $SOLPAY_ENV  (set MERCHANT_WALLET; keep ALLOW_MAINNET=false for devnet)
  2. Set your staff allowlist:
       zeroclaw config set peer_groups.whatsapp_staff.external_peers --config-dir "$CONFIG_DIR"
  3. Set WhatsApp + LLM secrets (masked, encrypted at rest):
       zeroclaw config set channels.whatsapp.default.access_token    --config-dir "$CONFIG_DIR"
       zeroclaw config set channels.whatsapp.default.app_secret      --config-dir "$CONFIG_DIR"
       zeroclaw config set channels.whatsapp.default.verify_token    --config-dir "$CONFIG_DIR"
       zeroclaw config set channels.whatsapp.default.phone_number_id --config-dir "$CONFIG_DIR"
       zeroclaw config set providers.models.openai.nlu.api_key       --config-dir "$CONFIG_DIR"
  4. Run it:  zeroclaw agent -a solpay --config-dir "$CONFIG_DIR"
EOF
ok "agent layer deployed and validated"
