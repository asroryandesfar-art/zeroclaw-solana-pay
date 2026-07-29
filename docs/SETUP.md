# Setup

`clone → configure → run`, verifiable in minutes. Sections 1–4 cover the
`solpay` helper end-to-end on devnet; section 5 deploys the WhatsApp + ZeroClaw
agent layer (validated against ZeroClaw v0.8.3).

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
make test        # 115 tests, offline and deterministic
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

## 5. Deploy the ZeroClaw agent (WhatsApp terminal)

Install [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) (v0.8.3+), then:

```bash
scripts/setup.sh          # builds solpay, deploys agent/ into ~/.zeroclaw, self-validates
```

`setup.sh` copies `agent/config.toml`, the `solpay` skill bundle, and the SOPs
into the config dir; sets an absolute `sop.sops_dir`; and runs
`zeroclaw skills list`, `zeroclaw sop validate`, and `zeroclaw doctor`. Then, one
time as the operator:

```bash
# 1. Locked money-path config (public values; no secrets):
$EDITOR ~/.zeroclaw/solpay.env           # set MERCHANT_WALLET; keep ALLOW_MAINNET=false

# 2. Staff allowlist (deny-by-default):
zeroclaw config set peer_groups.whatsapp_staff.external_peers --config-dir ~/.zeroclaw

# 3. WhatsApp + LLM secrets (masked input, encrypted at rest):
zeroclaw config set channels.whatsapp.default.access_token    --config-dir ~/.zeroclaw
zeroclaw config set channels.whatsapp.default.app_secret      --config-dir ~/.zeroclaw
zeroclaw config set channels.whatsapp.default.verify_token    --config-dir ~/.zeroclaw
zeroclaw config set channels.whatsapp.default.phone_number_id --config-dir ~/.zeroclaw
zeroclaw config set providers.models.openai.nlu.api_key       --config-dir ~/.zeroclaw

# 4. Run it:
zeroclaw agent -a solpay --config-dir ~/.zeroclaw
```

The two SOPs (`charge`, `verify-payments`) then drive the terminal: a staff
message like `Charge Table 4 25 USDC` creates an invoice + QR; a per-minute cron
re-checks pending invoices on-chain. Neither path calls the LLM for money logic.

## Troubleshooting

- **`invalid merchant wallet: … not on-curve`** — you pasted a token account or
  PDA; use your wallet's main public key.
- **`all RPC endpoints are unavailable` (exit 4)** — the RPC is unreachable or
  rate-limiting; set a provider URL in `SOLANA_RPC_PRIMARY`, or add
  `SOLANA_RPC_FALLBACK`.
- **`refusing to run on mainnet-beta…` (exit 3)** — intended interlock; set
  `ALLOW_MAINNET=true` only when you truly mean mainnet.
