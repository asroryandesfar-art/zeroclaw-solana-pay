#!/usr/bin/env bash
# Preflight: fail fast on a misconfigured environment before going live.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"
load_env

fail=0
note() { echo "  ✗ $*" >&2; fail=1; }

echo "==> Checking solpay"
SOLPAY="$(solpay_bin)"
[[ -n "$SOLPAY" ]] && ok "solpay: $SOLPAY" || { note "solpay not found (run scripts/setup.sh or 'make build')"; }

echo "==> Checking required configuration"
: "${SOLANA_CLUSTER:=devnet}"
[[ -n "${MERCHANT_WALLET:-}" ]] && ok "MERCHANT_WALLET set" || note "MERCHANT_WALLET is not set"
ok "cluster: $SOLANA_CLUSTER"
if [[ "$SOLANA_CLUSTER" == "mainnet-beta" && "${ALLOW_MAINNET:-false}" != "true" ]]; then
  note "mainnet-beta requires ALLOW_MAINNET=true"
fi

echo "==> Validating merchant wallet (via solpay)"
if [[ -n "$SOLPAY" && -n "${MERCHANT_WALLET:-}" ]]; then
  # A real create-url exercises on-curve wallet + mint resolution. Exit 0 = good.
  if MERCHANT_WALLET="$MERCHANT_WALLET" SOLANA_CLUSTER="$SOLANA_CLUSTER" \
       "$SOLPAY" create-url --amount "${MIN_CHARGE:-0.01}" --token "${PAYMENT_TOKEN:-USDC}" \
       --reference So11111111111111111111111111111111111111112 >/dev/null 2>/tmp/solpay_verify_env.err; then
    ok "merchant wallet + token/mint validated"
  else
    note "wallet/token validation failed: $(cat /tmp/solpay_verify_env.err)"
  fi
fi

echo "==> Checking RPC reachability"
RPC="${SOLANA_RPC_PRIMARY:-}"
if [[ -n "$RPC" ]]; then
  if curl -s -m 8 -X POST "$RPC" -H 'content-type: application/json' \
       -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q '"result":"ok"'; then
    ok "RPC healthy: $RPC"
  else
    note "RPC did not return healthy: $RPC"
  fi
else
  note "SOLANA_RPC_PRIMARY is not set"
fi

echo
if [[ "$fail" -eq 0 ]]; then echo "✓ environment OK"; else echo "✗ environment has problems (see above)"; exit 1; fi
