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
TOK="${TOKEN:-}"

# Fall back to the last invoice created by scripts/demo.sh.
if [[ -z "$REF" && -f "$STATE" ]]; then
  # shellcheck disable=SC1090
  source "$STATE"
  REF="${LAST_REFERENCE:-}"
  AMT="${AMT:-${LAST_AMOUNT:-}}"
  TOK="${TOK:-${LAST_TOKEN:-}}"
fi

[[ -n "$REF" ]] || die "no reference given and no saved invoice — run scripts/demo.sh first, or pass a reference"
AMT="${AMT:-25000000}"
TOK="${TOK:-USDC}"
: "${SOLANA_RPC_PRIMARY:=https://api.devnet.solana.com}"
# Small signature limit: SOL is matched by the merchant wallet's recent txs, and
# a busy wallet + public RPC will rate-limit (429) if we fetch too many. The
# payment is the most recent tx right after paying. Override with SIG_LIMIT.
: "${SIG_LIMIT:=6}"

# Use .env's MERCHANT_WALLET if present; otherwise fall back to the demo wallet.
: "${MERCHANT_WALLET:=$DEMO_MERCHANT_WALLET}"
export MERCHANT_WALLET

echo "verify: token=$TOK reference=$REF amount_base_units=$AMT recipient=$MERCHANT_WALLET rpc=$SOLANA_RPC_PRIMARY"
exec "$SOLPAY" --format human verify \
  --token "$TOK" \
  --reference "$REF" \
  --amount-base-units "$AMT" \
  --recipient "$MERCHANT_WALLET" \
  --signature-limit "$SIG_LIMIT" \
  --rpc "$SOLANA_RPC_PRIMARY"
