# ZeroClaw × Solana Bounty — Showcase Write-up

**Project:** ZeroClaw Solana Payment Assistant
**Bounty:** [Build Solana-native plugins for Zeroclaw](https://superteam.fun/earn/listing/zeroclaw/) (Superteam Brasil)
**Repo:** https://github.com/asroryandesfar-art/zeroclaw-solana-pay
**Release:** [v0.1.0](https://github.com/asroryandesfar-art/zeroclaw-solana-pay/releases/tag/v0.1.0)
**Demo video:** https://youtu.be/t4aPitLXOmo
**License:** MIT OR Apache-2.0

This write-up follows the submission format the bounty asks for: what it does,
who it's for, which ZeroClaw features it uses, what was built, its custody
tier and threat model, and links to reproduce it.

---

## What it does, and who it's for

A shop's staff messages their WhatsApp agent *"Charge Table 4, 25 USDC."*
Seconds later the customer's phone shows a QR. They scan it, pay in USDC or
SOL on Solana, and ~40 seconds later the staff channel reads
`Invoice #124 Paid ✅` — confirmed by the agent independently polling the
chain, not by trusting the customer's word.

It's for any operator who wants to accept Solana payments through a chat they
already run — a shop, a market stall, a small merchant — without running a
payment gateway, without a hosted SaaS middleman, and without ever handing an
AI agent a private key.

## Custody tier: **T1 (Build)**

Unsigned transactions only. `solpay` constructs a Solana Pay transfer-request
URL; the **customer's own wallet** builds and signs the actual transfer. The
agent process never holds, generates, or uses a private key — verified: there
is no `Keypair`, no `sign`, no transfer-submission code anywhere in the
codebase (see [`docs/PROMPT_INJECTION_TEST.md`](docs/PROMPT_INJECTION_TEST.md)).
Secrets held: **none** — `~/.zeroclaw/solpay.env` contains only a *public*
receiving key and public parameters.

## Which ZeroClaw features it uses (Tier 1 — stock release, zero plugins)

- **WhatsApp channel** — `dm_policy = "allowlist"` gate, deny-by-default.
- **SOP engine, deterministic mode** — two SOPs, no LLM round-trips at
  settlement: `charge` (channel-triggered) and `verify-payments`
  (cron-triggered, polling `getSignaturesForAddress` on the invoice
  reference — the exact pattern the bounty describes as the T1 idiom).
- **Skills** — three thin shell skills wrapping the `solpay` CLI
  (`create-invoice`, `send-qr`, `check-payment`); the model-visible tool
  schema exposes only `amount`/`token`/`message`, never a wallet.
- **Memory (SQLite)** — the invoice ledger, single source of truth.
- **Risk profile** — `allowed_commands` scoped to `solpay` only, high-risk
  commands blocked.

No plugin, no WASM, no fork of ZeroClaw — the release binary is used as-is.

## What (if anything) had to be built

One artifact outside ZeroClaw itself: `solpay`, a stateless Rust CLI
(`create-url`, `render-qr`, `verify`) that does the Solana-specific work —
Solana Pay URL construction, ATA derivation, and on-chain verification against
five independent checks (reference, exact mint, recipient, amount, commitment
level). It owns no database and no keys; state lives in ZeroClaw's memory.

## What makes it trustworthy

| Property | How |
|---|---|
| **Non-custodial** | The agent only ever holds a **public** receiving key. It never signs, never holds funds. Worst case on full compromise is a misleading invoice — not theft. |
| **LLM quarantined** | The model does exactly one thing: turn a message into a `{amount, token, message}` JSON intent. Recipient, mint, cluster, RPC, and commitment are locked config it cannot reach. |
| **Deterministic money path** | Both SOPs run in ZeroClaw **deterministic** mode; every money decision is made by the `solpay` Rust binary, not a prompt. |
| **Verify, don't trust** | Settlement requires an on-chain match on five checks: reference present, exact mint, merchant ATA, exact base-unit amount, commitment ≥ `confirmed` (`processed` rejected). Transient RPC errors → stay **PENDING**, never a false PAID. |
| **Fake-USDC defense** | The token mint is resolved from a compiled per-cluster table, never from message text. |
| **Mainnet interlock** | `ALLOW_MAINNET=false` by default; going live is a conscious two-value change. |

## Architecture (one screen)

```
WhatsApp ──▶ ZeroClaw ──▶ NLU (LLM, JSON only) ──▶ SOP "charge" (deterministic)
                                                      │
                                          solpay create-url + render-qr
                                                      │
                                   reply: QR + reference, ledger = PENDING
                                                      │
   cron (1/min) ──▶ SOP "verify-payments" ──▶ solpay verify ──▶ on-chain proof
                                                      │
                                        PENDING ──▶ PAID (atomic), notify staff
```

- `crates/solpay/` — stateless Rust helper (`create-url`, `render-qr`, `verify`).
  Integer-only money math, canonical `solana-pubkey` crypto, no DB, no keys.
- `agent/` — ZeroClaw `config.toml`, the `solpay` skill bundle, and the two SOPs.
- The invoice ledger lives in ZeroClaw memory (SQLite) — single source of truth.

## Prompt-injection test (required — funds are touched)

Two attacks were tried against the real, running code — not asserted, tested:

1. **Inject a `recipient` into the charge flow.** A malicious tool call with
   an extra `recipient` field pointing at an attacker wallet was run through
   ZeroClaw's actual argument-substitution algorithm, then the resulting
   command was executed against the real `solpay` binary. **Result: PASS** —
   the field has no placeholder to substitute into and is silently dropped;
   the invoice always pays the operator's locked `MERCHANT_WALLET`.
2. **Talk the agent into a "refund."** The codebase was searched for any
   signing/keypair/transfer capability. **Result: none exists** — there is no
   code path anywhere that could move funds, so there is nothing a social
   engineering attempt can hijack.

Full transcript, commands, and verdicts:
[`docs/PROMPT_INJECTION_TEST.md`](docs/PROMPT_INJECTION_TEST.md).

## Links to reproduce (config / SOPs / skills / code)

- Agent config: [`agent/config.toml`](agent/config.toml)
- Skills (model-visible tool schemas): [`agent/skills/solpay/`](agent/skills/solpay/)
- SOPs: [`agent/sops/charge/`](agent/sops/charge/), [`agent/sops/verify-payments/`](agent/sops/verify-payments/)
- Locked money-path config template (no secrets): [`agent/solpay.env.example`](agent/solpay.env.example)
- `solpay` source: [`crates/solpay/src/`](crates/solpay/src/)
- One-command deploy: [`scripts/setup.sh`](scripts/setup.sh)
- Step-by-step setup: [`docs/SETUP.md`](docs/SETUP.md)

## Verification (reproduce in minutes)

```bash
git clone https://github.com/asroryandesfar-art/zeroclaw-solana-pay
cd zeroclaw-solana-pay
make check          # fmt + clippy -D warnings + full test suite (offline)
make test-live      # optional: hits real devnet
scripts/setup.sh    # with ZeroClaw installed: deploys agent/ and self-validates
```

Both **USDC** and **SOL** have been verified end-to-end against real devnet
payments (proof signatures in the changelog / commit history).

## Engineering credibility

- Full test suite (offline + deterministic) plus real-devnet fixture tests and a
  live-devnet integration test.
- `clippy -D warnings` and `rustfmt` clean; CI runs fmt + clippy + tests + build.
- Zero `panic!`/`unwrap!`/`expect` in production code (test-only).
- Dual MIT/Apache-2.0 license; SECURITY, CONTRIBUTING, ADRs (0001–0005).
- Threat model and operations runbook documented.

## Honest scope

- Verified for **devnet**; mainnet real-money operation additionally needs a paid
  RPC, `finalized` commitment for large amounts, and the `ALLOW_MAINNET` interlock
  flipped deliberately.
- Solana Pay's transfer-request URL has no cluster field, so a wallet chooses the
  network; the demo uses a devnet-only mint, making a mainnet charge from the
  devnet QR impossible by construction.
