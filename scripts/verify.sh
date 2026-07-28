#!/usr/bin/env bash
# Verify an invoice's payment status — auto-loads .env, no manual `export` needed.
#
# Usage:
#   scripts/verify.sh                       # verify the LAST invoice from demo.sh
#   scripts/verify.sh <REFERENCE>           # verify a specific reference (amount 25 USDC)
#   scripts/verify.sh <REFERENCE> <AMOUNT_BASE_UNITS>
#
# This fixes "required configuration 'MERCHANT_WALLET' is not set": that error
# happens when `solpay verify` is run in a shell that never loaded .env. Here we
# source .env first, so MERCHANT_WALLET (and the RPC) are always present.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"
load_env

SOLPAY="$(solpay_bin)"
[[ -n "$SOLPAY" ]] || die "solpay not found — run 'make install' (or 'make build') first"

STATE="$ROOT/agent/data/last_invoice.env"
REF="${1:-}"
AMT="${2:-}"

# Fall back to the last invoice created by scripts/demo.sh.
if [[ -z "$REF" && -f "$STATE" ]]; then
  # shellcheck disable=SC1090
  source "$STATE"
  REF="${LAST_REFERENCE:-}"
  AMT="${AMT:-${LAST_AMOUNT:-}}"
fi

[[ -n "$REF" ]] || die "no reference given and no saved invoice — run scripts/demo.sh first, or pass a reference"
AMT="${AMT:-25000000}"
: "${SOLANA_RPC_PRIMARY:=https://api.devnet.solana.com}"

[[ -n "${MERCHANT_WALLET:-}" ]] || die "MERCHANT_WALLET is not set — add it to .env (this script auto-loads .env)"

echo "verify: reference=$REF amount_base_units=$AMT recipient=$MERCHANT_WALLET rpc=$SOLANA_RPC_PRIMARY"
exec "$SOLPAY" --format human verify \
  --reference "$REF" \
  --amount-base-units "$AMT" \
  --recipient "$MERCHANT_WALLET" \
  --rpc "$SOLANA_RPC_PRIMARY"
