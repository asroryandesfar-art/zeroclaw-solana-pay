#!/usr/bin/env bash
# End-to-end DEVNET demo of the solpay engine, no ZeroClaw required:
#   create invoice -> render QR -> verify (pending until paid).
#
# IMPORTANT: A Solana Pay URL has no cluster field (per the spec), so the QR
# cannot force the network — the wallet decides. Phantom defaults to MAINNET.
# The payer must switch Phantom to Devnet first (see the banner below).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"
load_env

SOLPAY="$(solpay_bin)"
[[ -n "$SOLPAY" ]] || die "solpay not found — run 'make build' or scripts/setup.sh first"

: "${SOLANA_CLUSTER:=devnet}"
: "${SOLANA_RPC_PRIMARY:=https://api.devnet.solana.com}"
: "${MERCHANT_WALLET:?set MERCHANT_WALLET (a devnet public key) in .env or the environment}"
: "${STORE_LABEL:=ZeroClaw Coffee}"
AMOUNT="${1:-25}"
TOKEN="${2:-USDC}"   # USDC (SPL) or SOL (native)

if [[ "$SOLANA_CLUSTER" != "devnet" ]]; then
  echo "WARNING: SOLANA_CLUSTER=$SOLANA_CLUSTER (not devnet). This demo is meant for devnet." >&2
fi

echo "==> 1) create-url  (amount=$AMOUNT $TOKEN, cluster=$SOLANA_CLUSTER)"
INVOICE_JSON="$("$SOLPAY" create-url --amount "$AMOUNT" --token "$TOKEN" \
  --label "$STORE_LABEL (Devnet)" --message "Demo table")"
echo "$INVOICE_JSON"

REFERENCE="$(printf '%s' "$INVOICE_JSON" | grep -o '"reference":"[^"]*"' | cut -d'"' -f4)"
URL="$(printf '%s' "$INVOICE_JSON" | grep -o '"url":"[^"]*"' | cut -d'"' -f4)"
# `mint` is null for native SOL — `|| true` keeps `set -e`/pipefail from aborting.
MINT="$(printf '%s' "$INVOICE_JSON" | grep -o '"mint":"[^"]*"' | cut -d'"' -f4 || true)"
AMOUNT_BASE="$(printf '%s' "$INVOICE_JSON" | grep -o '"amount_base_units":[0-9]*' | cut -d: -f2)"

# Save the invoice so `scripts/verify.sh` can check it with no arguments.
mkdir -p "$ROOT/agent/data"
printf 'LAST_REFERENCE=%s\nLAST_AMOUNT=%s\nLAST_TOKEN=%s\n' \
  "$REFERENCE" "$AMOUNT_BASE" "$TOKEN" > "$ROOT/agent/data/last_invoice.env"

echo
echo "==> 2) render-qr  -> /tmp/solpay-demo.png"
"$SOLPAY" render-qr --url "$URL" --out /tmp/solpay-demo.png
echo
echo "  +----------------------------------------------------------------------+"
echo "  |  DEVNET DEMO  --  set your wallet to Devnet BEFORE scanning           |"
echo "  |  A Solana Pay QR has no cluster field; the wallet picks the network.  |"
echo "  |  Phantom defaults to MAINNET, so switch it first:                     |"
echo "  |    Phantom -> Settings -> Developer Settings -> Testnet Mode = ON     |"
echo "  |    then select 'Solana Devnet'.                                       |"
echo "  |  This is a DEVNET invoice. Keep your wallet on Devnet — a wallet on   |"
echo "  |  Mainnet would attempt a REAL payment.                                |"
echo "  +----------------------------------------------------------------------+"
echo "  QR image:      /tmp/solpay-demo.png"
echo "  Pay with:      $TOKEN   (${MINT:-native SOL}, devnet-only)"
echo "  Watch on the DEVNET explorer:"
echo "    https://explorer.solana.com/address/$REFERENCE?cluster=devnet"

echo
echo "==> 3) verify  (reference=$REFERENCE)"
"$SOLPAY" --format human verify \
  --token "$TOKEN" --reference "$REFERENCE" --amount-base-units "$AMOUNT_BASE" \
  --rpc "$SOLANA_RPC_PRIMARY"

echo
echo "After paying on DEVNET, check status (any of these — no manual 'export' needed):"
echo "  scripts/verify.sh                       # verifies THIS invoice (auto-loads .env)"
echo "  scripts/verify.sh $REFERENCE"
echo "  $SOLPAY verify --token $TOKEN --reference $REFERENCE --amount-base-units $AMOUNT_BASE --recipient $MERCHANT_WALLET --rpc $SOLANA_RPC_PRIMARY"
