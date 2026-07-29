#!/usr/bin/env bash
# Create a LOCAL devnet SPL test token (a USDC-style stand-in) so the USDC/SPL
# flow can be tested WITHOUT the Circle faucet. You become the mint authority,
# so you can mint as much test token as you like. It uses the project's `--mint`
# override — no changes to the binary, nothing to do with real USDC.
#
# Requirements:
#   * solana + spl-token CLI:
#       sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
#   * a little devnet SOL for fees in your CLI wallet — get it at
#       https://faucet.solana.com   (the airdrop RPC is often rate-limited)
#
# Usage:
#   scripts/mint_test_token.sh [PAYER_WALLET] [AMOUNT] [SYMBOL]
#     PAYER_WALLET  the wallet you will PAY from in Phantom (receives the tokens).
#                   Omit to mint to your CLI wallet only.
#     AMOUNT        how many tokens to mint to the payer (default 100)
#     SYMBOL        allowlist symbol to use with solpay (default TUSD)
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

command -v solana >/dev/null || die 'solana CLI not found. Install: sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"'
command -v spl-token >/dev/null || die "spl-token CLI not found (part of the Solana tool suite)."

PAYER="${1:-}"
AMOUNT="${2:-100}"
SYMBOL="${3:-TUSD}"

solana config set --url https://api.devnet.solana.com >/dev/null
WALLET="$(solana address 2>/dev/null || true)"
[[ -n "$WALLET" ]] || die "no solana CLI keypair found. Create one: solana-keygen new"

echo "==> CLI wallet (mint authority + fee payer): $WALLET"
echo "    devnet SOL balance: $(solana balance 2>/dev/null || echo '?')"
solana airdrop 1 >/dev/null 2>&1 || \
  echo "    (airdrop unavailable — if creation fails, fund $WALLET at https://faucet.solana.com and re-run)"

echo "==> Creating a 6-decimal SPL token (USDC-style)…"
CREATE_OUT="$(spl-token create-token --decimals 6)"
echo "$CREATE_OUT"
MINT="$(printf '%s\n' "$CREATE_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{32,44}' | head -1)"
[[ -n "$MINT" ]] || die "could not parse the new mint address"

echo "==> Creating your CLI token account and minting a supply…"
spl-token create-account "$MINT" >/dev/null
spl-token mint "$MINT" "$((AMOUNT + 100))" >/dev/null

if [[ -n "$PAYER" ]]; then
  echo "==> Sending $AMOUNT $SYMBOL to your Phantom payer wallet $PAYER…"
  spl-token transfer "$MINT" "$AMOUNT" "$PAYER" --fund-recipient --allow-unfunded-recipient >/dev/null
  RECIP="$PAYER"
else
  RECIP="$WALLET"
fi

cat <<EOF

================= DONE =================
Test token (USDC-style, 6 decimals): $MINT
Holder of the tokens (the payer):    $RECIP

Test the SPL flow end-to-end with the assistant:
  export TOKEN_ALLOWLIST=USDC,SOL,$SYMBOL
  export MERCHANT_WALLET=<your merchant wallet>

  solpay create-url --token $SYMBOL --mint $MINT --amount 1        # → QR
  # open the QR and pay 1 $SYMBOL from Phantom (Devnet) with the wallet above
  scripts/verify.sh <reference>   # or:
  solpay verify --token $SYMBOL --mint $MINT --amount-base-units 1000000 \\
    --reference <reference> --recipient \$MERCHANT_WALLET

This exercises the exact SPL path (reference + ATA + exact-mint + amount) that
real USDC uses — with a token you fully control, no external faucet.
EOF
