# ZeroClaw × Solana — Bounty Submission

**Project:** ZeroClaw Solana Payment Assistant
**Track:** Build Solana-native plugins/capabilities for ZeroClaw
**Repo:** https://github.com/asroryandesfar-art/zeroclaw-solana-pay
**Release:** [v0.1.0](https://github.com/asroryandesfar-art/zeroclaw-solana-pay/releases/tag/v0.1.0)
**Demo video:** https://youtu.be/t4aPitLXOmo
**License:** MIT OR Apache-2.0

---

## One line

Turn WhatsApp into a **non-custodial** Solana Pay terminal: a staff member texts
`Charge Table 4 25 USDC`, the agent replies with a QR, the customer pays in
USDC or SOL, and the agent confirms **PAID** by verifying the transaction
on-chain — with **no LLM anywhere on the money path**.

## Why it fits the bounty

- **Tier-1 ZeroClaw, no fork.** Built entirely on ZeroClaw built-ins — WhatsApp
  channel, SOP engine (deterministic mode), cron, and memory — plus one small,
  auditable Rust CLI invoked as a skill. No compiled plugin, no patched runtime.
- **Solana-native.** Implements the Solana Pay transfer-request spec (URL + QR),
  ATA derivation, and on-chain settlement verification over JSON-RPC, for both
  SPL **USDC** and native **SOL**.
- **Validated on a real ZeroClaw runtime** (v0.8.3, schema_version 3): the
  config, skills, and SOPs load and pass `zeroclaw skills list`,
  `zeroclaw sop validate`, and `zeroclaw doctor`.

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
