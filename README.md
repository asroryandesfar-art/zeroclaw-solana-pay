# ZeroClaw Solana Payment Assistant

[![CI](https://github.com/asroryandesfar-art/zeroclaw-solana-pay/actions/workflows/ci.yml/badge.svg)](https://github.com/asroryandesfar-art/zeroclaw-solana-pay/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)

**Turn WhatsApp into a Solana payment terminal — non-custodial.**

A shop messages the agent *"Charge Table 4 for 25 USDC."* Seconds later it gets back
a QR code. The customer scans and pays in USDC on Solana. The agent watches the
chain, confirms the payment, and replies **`Invoice #124 Paid ✅`**.

> **The agent can take money, but it can never touch it.** There is no private
> key anywhere in this system — it only ever needs a *receiving public key*. The
> agent cannot sign, move, or hold funds. That single property removes most of
> the threat model.

Built for the ZeroClaw × Solana bounty on [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw)
and [Solana Pay](https://docs.solanapay.com/spec), using ZeroClaw built-ins (channels,
SOP, cron, memory) plus one small, auditable Rust helper.

---

## The flow

```
  WhatsApp                Agent (ZeroClaw)              Solana
  ────────                ────────────────              ──────
  "Charge Table 4    ─►   understand + validate
   25 USDC"               build Solana Pay URL
                          render QR            ─────►   (customer scans & pays USDC)
  ◄── QR image
                          poll for payment     ◄─────   getSignaturesForAddress
                          verify (5 checks)              getTransaction
  ◄── "Invoice #124       └─ mint · amount · recipient
      Paid ✅"               · reference · confirmed
```

## Why it is safe (in one picture)

The LLM only reads language. Every decision about money is deterministic code.

```
     UNTRUSTED / PROBABILISTIC             TRUSTED / DETERMINISTIC
   ┌──────────────────────────┐         ┌────────────────────────────┐
   │ LLM: message → intent JSON│  intent │ invoice state machine      │
   │ LLM: friendly reply text  │ ───────►│ Solana Pay URL + QR        │
   └──────────────────────────┘         │ on-chain verifier (RPC)    │
                                         │ never signs · no private key│
                                         └────────────────────────────┘
```

## Features

- **Non-custodial** — receiving public key only; no signing, no fund custody.
- **LLM quarantined** — the model never computes an amount or decides "is this paid?"
- **Tier-1 ZeroClaw** — channels, SOP (deterministic mode), cron, and memory built-ins; the only bespoke code is a stateless Rust CLI.
- **USDC or native SOL** — invoice and verify in USDC (SPL) or SOL (`--token SOL`); both verified against real devnet payments.
- **Correct by construction** — integer-only money math, exact-mint check (anti fake-USDC), exact-amount check, reference-based replay resistance, `confirmed`/`finalized` gating.
- **Resilient** — RPC failover + bounded retries with jitter; an unreachable node keeps an invoice *pending*, never falsely *paid* or *failed*.
- **Reproducible** — pinned toolchain, committed lockfile, one `make` surface, 115 tests.

---

## Quickstart (devnet)

```bash
git clone <repo> && cd zeroclaw-solana-pay
cp .env.example .env          # a demo MERCHANT_WALLET is pre-filled; replace it with your own for real use
make test                     # 115 tests, offline & deterministic
make install                  # puts `solpay` on your PATH

# Load your config into the shell (MERCHANT_WALLET, SOLANA_CLUSTER, RPC…)
set -a; source .env; set +a

# Create an invoice → note the "url" and "reference" in the JSON output
solpay create-url --amount 25 --token USDC --message "Table 4"

# Render its QR (paste the "url" from above)
solpay render-qr --url 'solana:...' --out /tmp/qr.png

# Verify payment (returns paid | pending | mismatch). `verify.sh` auto-loads .env
# so you never need to export anything; pass the reference (or none for the last one).
scripts/verify.sh <ref>
```

Or just run **`scripts/demo.sh`**, which does create → QR → verify against devnet
for you. Full walkthrough (including WhatsApp + the ZeroClaw agent) is in
[`docs/SETUP.md`](docs/SETUP.md).

---

## Paying in USDC or SOL (devnet, end-to-end)

Both are verified against real devnet payments. Use a **dedicated merchant
wallet** you control (set `MERCHANT_WALLET` in `.env`) and switch **Phantom to
Devnet** (Settings → Developer Settings → Testnet Mode → Solana Devnet).

**USDC** — reference-bound (the wallet embeds the Solana Pay reference):

```bash
# fund the payer wallet with devnet USDC (mint 4zMMC9…) via faucet.circle.com
scripts/demo.sh 1 USDC          # create invoice → QR (/tmp/solpay-demo.png)
# scan the QR in Phantom (Devnet) and approve
scripts/verify.sh               # → PAID ✅
```

**SOL** — native (no `spl-token` in the URL):

```bash
# fund the payer wallet with devnet SOL via faucet.solana.com
scripts/demo.sh 0.05 SOL        # create invoice → QR
# scan the QR in Phantom (Devnet) and approve
scripts/verify.sh               # → PAID ✅
```

> **How verification differs:** USDC is matched by the unique Solana Pay
> **reference** (Phantom includes it for SPL tokens). Phantom does **not** attach
> the reference to *native SOL* transfers, so SOL is matched by the exact lamport
> amount credited to the **merchant wallet** — use a dedicated (non-busy) wallet,
> and USDC when you need strict per-invoice binding.

### No devnet USDC? (Circle faucet blocked / unavailable)

Devnet USDC comes from Circle's faucet at **https://faucet.circle.com** (select
Solana Devnet). If it won't open, it's usually an **ISP DNS block** — switch your
DNS to `1.1.1.1` (or use a VPN); the faucet itself is up.

To test the SPL flow **without any faucet**, mint a local USDC-style test token
(you become the mint authority) and use the `--mint` override:

```bash
scripts/mint_test_token.sh <your-phantom-payer-wallet>   # needs solana + spl-token CLI
# → prints a TEST MINT and the exact solpay commands to use it
```

This exercises the identical SPL path (reference + ATA + exact-mint + amount) that
real USDC uses, with a token you fully control.

### Wallet notes (devnet)

- **Solflare** does not properly support Solana Pay on **devnet** (it rejects the
  QR as an "invalid address" even though the URI is valid). Use **Phantom** for
  devnet Solana Pay.
- **Phantom (Android)** may **not display** a devnet USDC balance: Circle's devnet
  USDC (`4zMMC9…`) has no on-chain Metaplex metadata and isn't in Phantom's devnet
  token list. This is a wallet UI limitation, **not** a payment problem — a
  `simulateTransaction` of the exact Solana Pay USDC transfer from a funded ATA
  succeeds (`CreateIdempotent` + `TransferChecked`, `err: null`). Scanning the QR
  and approving still moves the funds. If a wallet refuses to spend a token it
  doesn't display, pay from a wallet that shows it, or use the **SOL** flow.
- **Public devnet RPC rate-limits.** SOL verification scans the merchant wallet's
  recent transactions; on a **busy** wallet with the free `api.devnet.solana.com`,
  rapid/repeated calls can return `all RPC endpoints are unavailable` (HTTP 429).
  It's transient — wait a few seconds and re-run `scripts/verify.sh`, keep a small
  `--signature-limit`, or set a paid RPC in `SOLANA_RPC_PRIMARY`. **USDC**
  verification is reference-based (a single lookup) and does not hit this.

---

## The `solpay` helper

A single stateless binary. The agent never does math on money; it calls these.

| Command | Does | Touches network |
|---|---|---|
| `create-url` | build a reference + Solana Pay URL for an amount | no |
| `render-qr` | rasterize a `solana:` URL to a PNG | no |
| `verify` | decide `paid` / `pending` / `mismatch` from the chain | yes (only this) |

Every command supports `--help` and `--format json` (default) or `--format human`.

### Output schemas (stable — fields are only ever added)

```jsonc
// create-url
{ "reference","url","recipient","mint","token","cluster",
  "amount_base_units","amount_ui","label","message" }

// render-qr
{ "image_path","format","size_bytes","modules","pixel_size" }

// verify
{ "status": "paid|pending|mismatch", "signature": "…|null",
  "slot": 123|null, "reason": "…|null" }
```

### Exit codes (the contract ZeroClaw SOPs branch on)

| Code | Meaning | SOP reaction |
|---|---|---|
| `0` | success | read the JSON on stdout |
| `2` | invalid input | reject to the user (bad amount/reference/token) |
| `3` | config error | halt; operator misconfiguration |
| `4` | RPC / transient | keep the invoice **pending**; retry next tick |
| `5` | internal error | alert; leave state untouched |

---

## How verification works

`verify` declares **paid** only when *all* of these hold — none of them taken
from the message or the LLM:

1. the transaction includes this invoice's unique **reference**,
2. the token is the **exact USDC mint** (a token merely *named* "USDC" is rejected) — for a SOL invoice, the funds are native lamports,
3. funds landed in the **merchant's associated token account** (USDC) or the **merchant wallet** (SOL),
4. the **amount** meets or exceeds the expected base units,
5. commitment is **≥ `confirmed`** (never `processed`), and the tx succeeded.

An unknown/unreachable chain is treated as *pending*, never as a negative.
See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and
[`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md).

---

## Repository layout

```
crates/solpay/     the deterministic, non-custodial Rust helper (money lives here)
  src/money.rs           integer-only amounts
  src/domain/            invoice state machine + validation
  src/solana/            pubkey · ATA · reference · pay_url · rpc · verify
  src/{config,error,output,qr,cli}.rs   CLI surface
  tests/                 CLI + real-devnet fixtures + parse→decide
agent/             ZeroClaw agent layer (validated against ZeroClaw v0.8.3)
  config.toml            schema_version 3 config (provider, channel, risk, memory)
  skills/solpay/         create-invoice · send-qr · check-payment (SKILL.toml)
  sops/                  charge · verify-payments (SOP.toml + SOP.md)
  solpay.env.example     locked money-path config (→ ~/.zeroclaw/solpay.env)
docs/              architecture, threat model, configuration, setup, ADRs
```

The `agent/` layer is not inferred from docs — it loads and validates on a real
ZeroClaw runtime (`zeroclaw skills list`, `zeroclaw sop validate`,
`zeroclaw doctor`); `scripts/setup.sh` deploys and re-checks it. See
[ADR 0005](docs/adr/0005-validated-against-zeroclaw.md).

## Development

```bash
make check     # fmt-check + clippy -D warnings + tests (what CI runs)
make test-live # also run the ignored network tests against devnet
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), matching
ZeroClaw. Contributions are accepted under the same terms — see
[`CONTRIBUTING.md`](CONTRIBUTING.md).
