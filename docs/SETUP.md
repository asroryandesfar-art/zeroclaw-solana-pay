# Setup

`clone → configure → run`, verifiable in minutes. This document covers the
`solpay` helper end-to-end on devnet. The full WhatsApp + ZeroClaw agent wiring
is added in the agent phase.

## Prerequisites

- **Rust** (stable) — the pinned toolchain in `rust-toolchain.toml` is used
  automatically by `rustup`.
- A **Solana devnet RPC** URL. The public `https://api.devnet.solana.com` works
  for a demo; a provider (e.g. Helius) is recommended to avoid rate limits.
- (Agent phase) **ZeroClaw**, and a WhatsApp Cloud API app *or* Web-mode session.

No Solana CLI and no keypair are required for the helper — it never signs.

## 1. Clone and configure

```bash
git clone <repo> && cd zeroclaw-solana-pay
cp .env.example .env
```

Edit `.env` and set at least `MERCHANT_WALLET` (a base58, on-curve **public**
key — your receiving wallet). Keep `SOLANA_CLUSTER=devnet` and
`ALLOW_MAINNET=false` while testing.

## 2. Verify the build

```bash
make test        # 107 tests, offline and deterministic
make lint        # clippy -D warnings
make install     # installs `solpay` to ~/.cargo/bin
```

## 3. Devnet walkthrough

```bash
# Create an invoice (deterministic if you pass --reference).
solpay create-url --amount 25 --token USDC --message "Table 4"

# → copy "url" and "reference" from the JSON.

# Render the QR.
solpay render-qr --url '<url>' --out /tmp/qr.png
open /tmp/qr.png     # or xdg-open

# Before payment, verify returns pending.
solpay verify --reference <reference> --amount-base-units 25000000 \
  --rpc https://api.devnet.solana.com
# → {"status":"pending",...}
```

To see a full **paid** cycle you pay the QR from a devnet wallet holding
devnet USDC (mint `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`). After the
transfer confirms, `verify` returns `{"status":"paid","signature":...}`.

> Tip: run the live network test with `make test-live` — it creates a fresh
> reference and confirms `verify` reaches devnet and returns `pending`.

## 4. Human-readable mode

Add `--format human` to any command for friendly text instead of JSON.

## Troubleshooting

- **`invalid merchant wallet: … not on-curve`** — you pasted a token account or
  PDA; use your wallet's main public key.
- **`all RPC endpoints are unavailable` (exit 4)** — the RPC is unreachable or
  rate-limiting; set a provider URL in `SOLANA_RPC_PRIMARY`, or add
  `SOLANA_RPC_FALLBACK`.
- **`refusing to run on mainnet-beta…` (exit 3)** — intended interlock; set
  `ALLOW_MAINNET=true` only when you truly mean mainnet.
