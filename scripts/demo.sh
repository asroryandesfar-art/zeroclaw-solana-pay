#!/usr/bin/env bash
# End-to-end devnet demo of the solpay engine, no ZeroClaw required:
#   create invoice -> render QR -> verify (pending until paid).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"
load_env

SOLPAY="$(solpay_bin)"
[[ -n "$SOLPAY" ]] || die "solpay not found — run 'make build' or scripts/setup.sh first"

: "${SOLANA_CLUSTER:=devnet}"
: "${SOLANA_RPC_PRIMARY:=https://api.devnet.solana.com}"
: "${MERCHANT_WALLET:?set MERCHANT_WALLET (a devnet public key) in .env or the environment}"
AMOUNT="${1:-25}"

echo "==> 1) create-url  (amount=$AMOUNT USDC, cluster=$SOLANA_CLUSTER)"
INVOICE_JSON="$("$SOLPAY" create-url --amount "$AMOUNT" --token USDC --message "Demo table")"
echo "$INVOICE_JSON"

REFERENCE="$(printf '%s' "$INVOICE_JSON" | grep -o '"reference":"[^"]*"' | cut -d'"' -f4)"
URL="$(printf '%s' "$INVOICE_JSON" | grep -o '"url":"[^"]*"' | cut -d'"' -f4)"
AMOUNT_BASE="$(printf '%s' "$INVOICE_JSON" | grep -o '"amount_base_units":[0-9]*' | cut -d: -f2)"

echo
echo "==> 2) render-qr  -> /tmp/solpay-demo.png"
"$SOLPAY" render-qr --url "$URL" --out /tmp/solpay-demo.png
echo "  open /tmp/solpay-demo.png and pay it from a devnet wallet holding devnet USDC"

echo
echo "==> 3) verify  (reference=$REFERENCE)"
"$SOLPAY" --format human verify \
  --reference "$REFERENCE" --amount-base-units "$AMOUNT_BASE" \
  --rpc "$SOLANA_RPC_PRIMARY"

echo
echo "Re-run this verify after paying to see PAID:"
echo "  $SOLPAY verify --reference $REFERENCE --amount-base-units $AMOUNT_BASE --rpc $SOLANA_RPC_PRIMARY"
