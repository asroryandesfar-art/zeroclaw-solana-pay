# ZeroClaw Solana Pay — Supporting Material

**Turn WhatsApp into a non-custodial Solana payment terminal.**

| | |
|---|---|
| **Project** | ZeroClaw Solana Payment Assistant |
| **Track** | Build Solana-native capabilities for ZeroClaw |
| **Repository** | https://github.com/asroryandesfar-art/zeroclaw-solana-pay |
| **Release** | [v0.1.0](https://github.com/asroryandesfar-art/zeroclaw-solana-pay/releases/tag/v0.1.0) |
| **Demo video** | https://youtu.be/t4aPitLXOmo |
| **License** | MIT OR Apache-2.0 |
| **CI** | [![CI](https://github.com/asroryandesfar-art/zeroclaw-solana-pay/actions/workflows/ci.yml/badge.svg)](https://github.com/asroryandesfar-art/zeroclaw-solana-pay/actions/workflows/ci.yml) |
| **Document version** | Prepared 2026-07-30, against commit `1183475` |

---

## Table of Contents

1. [One-Page Project Overview](#1-one-page-project-overview)
2. [Executive Summary](#2-executive-summary)
3. [Architecture Documentation](#3-architecture-documentation)
4. [Payment Flow Diagrams](#4-payment-flow-diagrams)
5. [System Diagrams](#5-system-diagrams)
6. [Feature Overview](#6-feature-overview)
7. [Technical Documentation](#7-technical-documentation)
8. [API Overview (CLI Interface)](#8-api-overview-cli-interface)
9. [Installation Guide](#9-installation-guide)
10. [Quick Start](#10-quick-start)
11. [Security Considerations](#11-security-considerations)
12. [Known Limitations](#12-known-limitations)
13. [Roadmap](#13-roadmap)
14. [FAQ](#14-faq)
15. [Demo Walkthrough](#15-demo-walkthrough)
16. [Screenshots Section](#16-screenshots-section)
17. [Judge Notes](#17-judge-notes)
18. [Appendix — Links & References](#18-appendix--links--references)

---

## 1. One-Page Project Overview

**Problem.** Accepting a Solana payment from a chat interface today means either
building a custodial bot that holds a private key (a liability and a target), or
wiring a payment SDK by hand into an agent framework with no guardrails against
the LLM making a mistake with money.

**Solution.** ZeroClaw Solana Pay turns a WhatsApp conversation into a Solana Pay
terminal, built almost entirely on **ZeroClaw's built-in capabilities** — the
WhatsApp channel, the SOP orchestration engine (in deterministic mode), cron,
and memory — plus **one small, auditable Rust CLI** (`solpay`) that does all the
money math and on-chain verification.

**How it works, in one sentence.** A staff member sends *"Charge Table 4 25
USDC"*; the LLM extracts a structured intent (nothing more); a deterministic
ZeroClaw SOP calls `solpay` to build a Solana Pay QR; the customer pays; a
cron-driven SOP calls `solpay verify` every minute until the payment is
confirmed on-chain; the staff member gets `Invoice #124 Paid ✅`.

**Why it is trustworthy.**
- **Non-custodial** — the system stores only a public receiving key. It never
  signs a transaction and never holds funds.
- **LLM quarantined** — the model only turns language into a JSON intent
  (`amount`, `token`, `message`) and writes reply text. Every money decision —
  the amount, the wallet, the mint, the payment verdict — is deterministic Rust,
  never a model output.
- **Verified on-chain, not trusted** — a payment is marked `PAID` only after five
  independent checks pass: reference match, exact token mint, correct recipient,
  exact amount, and commitment level.

**Status.** The `solpay` engine (money core + CLI) is complete and covered by
**115 automated tests**, verified end-to-end against **real Solana devnet
payments in both USDC and native SOL**. The ZeroClaw agent layer (config,
skills, SOPs) has been **validated against a real ZeroClaw v0.8.3 runtime** —
not just written to documentation. CI is green. A 3-minute demo video shows the
full flow, including a live devnet payment settling to `PAID`.

**Tech stack.** Rust (stable toolchain, workspace with one crate), ZeroClaw
(WhatsApp channel, SOP engine, cron, SQLite memory), Solana JSON-RPC, Solana Pay
transfer-request spec.

---

## 2. Executive Summary

ZeroClaw Solana Pay is a submission for the ZeroClaw × Solana bounty track,
demonstrating a production-shaped pattern for letting an AI agent handle money
without ever being trusted with it.

**The core design bet.** Most of the risk in "AI agent + payments" comes from
two places: the agent holding funds, and the agent (or its LLM) making a
decision about money. This project removes both. The agent is **non-custodial**
by construction — it only ever needs a public key, so a fully compromised host
cannot move a single lamport. The **LLM is quarantined** to language — it turns
a WhatsApp message into a validated JSON intent and writes the reply text; it
never computes an amount, never picks a wallet, and never decides "is this
paid?". That decision is made by a small, deterministic, unit-tested Rust binary
that independently checks the chain.

**Built on ZeroClaw, not around it.** The project deliberately uses only
Tier-1 ZeroClaw building blocks — the WhatsApp channel, the Standard Operating
Procedure (SOP) engine running in **deterministic mode** (no LLM round-trips at
settlement), the cron scheduler, and the SQLite-backed memory layer as the
invoice ledger. The only bespoke code is `solpay`, a stateless CLI with three
subcommands. This keeps the integration surface small, auditable, and portable
to any ZeroClaw installation.

**Evidence, not claims.** Every architectural claim in this document is backed
by something reproducible: 115 automated tests (unit, CLI black-box, real-devnet
fixtures, and a live-network integration test); `clippy -D warnings` and
`rustfmt` clean; a green CI pipeline; real devnet transactions settling to
`PAID` in both USDC and SOL; and — the piece most teams skip — the ZeroClaw
agent configuration, skill definitions, and SOP definitions have been
**installed against a real ZeroClaw v0.8.3 binary** and validated with its own
tooling (`zeroclaw doctor`, `zeroclaw skills audit`, `zeroclaw sop validate`),
not merely written to match documentation.

**Hackathon fit.** The submission demonstrates: a real Solana-native integration
(Solana Pay spec, ATA derivation, on-chain settlement verification); genuine use
of ZeroClaw as a Tier-1 platform rather than a wrapper; a security posture that
is unusually rigorous for a hackathon submission (explicit threat model, ADRs,
mainnet interlock); and full reproducibility from a clean clone.

**Honest scope.** This is verified on **devnet**. Mainnet operation is
supported by the same code path but requires an explicit, documented interlock
(`ALLOW_MAINNET=true`) and additional operator steps (paid RPC, `finalized`
commitment for high-value invoices) before it should handle real funds — see
[Known Limitations](#12-known-limitations).

---

## 3. Architecture Documentation

### Design goal

> An AI agent can accept a payment, but no part of the system can move or hold
> funds, and the LLM can never make a decision about money.

### Two planes

The system is split into an **untrusted, probabilistic plane** (language) and a
**trusted, deterministic plane** (money), meeting only at a validated JSON
contract.

```
        UNTRUSTED / PROBABILISTIC              TRUSTED / DETERMINISTIC
      ┌────────────────────────────┐        ┌──────────────────────────────┐
WA ─► │ ZeroClaw channel (WhatsApp)│        │ solpay (stateless Rust CLI)  │
      │ sender allowlist gate      │        │  money.rs   integer amounts  │
      │ LLM: message → intent JSON │ intent │  domain/    state + validate │
      │ LLM: friendly reply text   │ ─────► │  solana/    url · qr · verify│
      └────────────────────────────┘  JSON  │  never signs · no privkey    │
                    ▲                        └──────────────┬───────────────┘
                    │ reply / QR                            │ reads only
                    └───────────────── ZeroClaw ────────────┘
                     SOP (deterministic) · cron · memory (ledger)
```

If the LLM's output fails schema or bounds validation, the request is rejected
before any Solana logic runs. Prompt injection therefore cannot set the
recipient wallet, the token, or bypass amount limits — those values come from
**locked configuration**, never from the message.

### Layers (each replaceable, single responsibility)

| Layer | Where | Responsibility |
|---|---|---|
| Channel / adapter | ZeroClaw gateway (WhatsApp) | receive/send messages + media |
| Authorization | ZeroClaw channel `dm_policy=allowlist` + `peer_groups` | only staff numbers can charge |
| Intent (NLU) | ZeroClaw + LLM | `"Charge Table 4 25 USDC"` → intent JSON |
| Orchestration | ZeroClaw SOP (deterministic mode) | sequence steps; **no LLM on the money path** |
| Domain | `solpay` `domain/` | invoice state machine + validation (pure) |
| Money / Solana | `solpay` `money`, `solana/` | amounts, Pay URL, QR, on-chain verify |
| Persistence | ZeroClaw memory (SQLite) | invoice ledger (single source of truth) |
| Scheduler | ZeroClaw cron | poll pending invoices; expire stale ones |

The channel, SOP, cron, and memory layers are ZeroClaw built-ins. The only
bespoke code is `solpay`, and it is **stateless** — it holds no database and no
private keys.

### The fetch/decide split (why verification is trustworthy)

Inside `solpay`, network I/O and the payment verdict are deliberately separated:

- `solana/rpc.rs` **fetches** — JSON-RPC over a small blocking HTTP client
  behind an `HttpTransport` trait, with ordered endpoint failover and bounded,
  jittered retries.
- `solana/verify.rs` **decides** — a pure function of already-fetched evidence.

Because the verdict never touches the network, every case is tested offline
against real-devnet fixtures, and a flaky or hostile RPC node cannot perturb
the money decision — it can only delay it (see [Security Considerations](#11-security-considerations)).

### Repository layout

```
crates/solpay/     the deterministic, non-custodial Rust helper (money lives here)
  src/money.rs           integer-only amounts
  src/domain/            invoice state machine + validation
  src/solana/            pubkey · ata · commitment · model · pay_url · reference · rpc · verify
  src/{config,error,output,qr,cli,lib,main}.rs   CLI surface
  tests/                 CLI black-box + real-devnet fixtures + parse→decide
agent/             ZeroClaw agent layer (validated against ZeroClaw v0.8.3)
  config.toml            schema_version 3 config (provider, channel, risk, memory)
  skills/solpay/         create-invoice · send-qr · check-payment (SKILL.toml)
  sops/                  charge · verify-payments (SOP.toml + SOP.md)
  solpay.env.example     locked money-path config (→ ~/.zeroclaw/solpay.env)
docs/              architecture, threat model, configuration, setup, operations, ADRs
```

Full narrative version: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## 4. Payment Flow Diagrams

### 4.1 The `charge` flow (staff → invoice → QR)

```
Staff (WhatsApp)        ZeroClaw SOP "charge"         solpay            Memory (ledger)
      │                        │                         │                    │
      ├─ "Charge Table 4 ────► │                         │                    │
      │   25 USDC"             │─ allowlist gate         │                    │
      │                        │─ LLM: message → intent  │                    │
      │                        │   JSON (amount/token/   │                    │
      │                        │   message only)         │                    │
      │                        │── create-url ──────────►│                    │
      │                        │   (amount, token;       │                    │
      │                        │    recipient/mint       │                    │
      │                        │    locked from env)     │                    │
      │                        │◄── {reference, url} ────│                    │
      │                        │───────────── write invoice PENDING ─────────►│
      │                        │── render-qr ────────────►│                    │
      │                        │◄── {image_path} ─────────│                    │
      │◄── QR image + ─────────┤                         │                    │
      │   "Invoice #124 —      │                         │                    │
      │    scan to pay"        │                         │                    │
```

*(Mermaid source, renders as a diagram on GitHub: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#the-charge-flow).)*

### 4.2 The `verify-payments` flow (cron → settlement)

```
Cron (ZeroClaw, 1/min)     Memory (ledger)         solpay              Staff (WhatsApp)
        │                       │                     │                      │
        ├─ list invoices ──────►│                     │                      │
        │  where status=PENDING │                     │                      │
        │                       │                     │                      │
        │  for each pending invoice:                  │                      │
        │    expired? ──────────► PENDING → EXPIRED, skip                    │
        │                       │                     │                      │
        │── verify --reference --amount-base-units ──►│                      │
        │                       │                     │                      │
        │   exit 0, status=paid │                     │                      │
        │──────────────────────► PENDING → PAID ──────┼─────────────────────►│
        │                       │  (guarded, once)     │      "Invoice #124  │
        │                       │                     │        Paid ✅"      │
        │   exit 0, status=mismatch                    │                      │
        │──────────────────────► PENDING → FAILED      │                      │
        │                       │  (reason recorded)   │                      │
        │   exit 4 (RPC transient)                     │                      │
        │── leave PENDING, retry next tick ────────────┤                      │
```

*(Mermaid source: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#the-verify_payments-flow-cron).)*

### 4.3 On-chain verification — the five checks

A payment is declared **paid** only when *all five* hold — none of them taken
from the message or the LLM:

1. the transaction includes this invoice's unique **reference**,
2. the token is the **exact mint** for the expected symbol (a token merely
   *named* "USDC" is rejected) — for a SOL invoice, the funds are native lamports,
3. funds landed in the **merchant's associated token account** (USDC) or the
   **merchant wallet** (SOL),
4. the **amount** meets or exceeds the expected base units,
5. commitment is **≥ `confirmed`** (never `processed`), and the transaction
   succeeded (`err: null`).

An unreachable or erroring RPC is treated as **unknown**, never as a negative —
the invoice stays `PENDING` and is retried.

---

## 5. System Diagrams

### 5.1 Invoice state machine

```
                 ┌────────────────────────────────────────┐
                 │           charge SOP writes ledger      │
                 ▼                                          │
   (start) ──► PENDING ──────────────────────────────────────┘
                 │   │
                 │   └── TTL elapsed / attempts exhausted ──► EXPIRED  (terminal)
                 │
                 ├── verify: all 5 checks pass ──► PAID ── confirmation sent ──► SETTLED  (terminal)
                 │
                 └── verify: mismatch (mint/amount/recipient) ──► FAILED  (terminal)
```

Transitions are driven **only** by deterministic code and on-chain evidence.
Terminal states are frozen, and a settlement is applied only while the invoice
is still `PENDING` — so a confirmation is sent exactly once (idempotency).
*(Mermaid `stateDiagram-v2` source: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#invoice-state-machine).)*

### 5.2 Trust boundary map

```
 TRUSTED                                          UNTRUSTED (validated before use)
 ───────                                          ─────────────────────────────────
 host + filesystem holding ~/.zeroclaw/solpay.env  WhatsApp message content
 agent/config.toml + ZeroClaw encrypted secrets    LLM output (schema + bounds checked)
 whoever controls the staff allowlist              RPC responses (re-verified on-chain)
 the compiled solpay binary                        every customer / payer
```

Full table with mitigations: [Security Considerations](#11-security-considerations)
and [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md).

### 5.3 Module dependency sketch (solpay)

```
cli.rs / main.rs
   │
   ├── config.rs ───────────► (env + CLI flag resolution, fail-fast validation)
   │
   ├── domain/
   │     ├── invoice.rs ────► state machine + idempotency guard
   │     └── validation.rs ─► charge limits, allowlist
   │
   ├── money.rs ─────────────► integer-only decimal ↔ base-units math
   │
   └── solana/
         ├── pubkey.rs ──────► base58 + on-curve checks
         ├── ata.rs ─────────► associated-token-account derivation
         ├── reference.rs ───► unique, on-curve payment references
         ├── pay_url.rs ─────► Solana Pay transfer-request URL builder
         ├── commitment.rs ──► ordered commitment levels
         ├── model.rs ───────► evidence types (signatures, balance deltas)
         ├── rpc.rs ─────────► JSON-RPC fetch (failover + retries) — FETCH
         └── verify.rs ──────► pure payment verdict from evidence — DECIDE
```

---

## 6. Feature Overview

- **Non-custodial** — receiving public key only; no signing, no fund custody.
- **LLM quarantined** — the model never computes an amount or decides "is this
  paid?".
- **Tier-1 ZeroClaw** — channels, SOP (deterministic mode), cron, and memory
  built-ins; the only bespoke code is a stateless Rust CLI.
- **USDC or native SOL** — invoice and verify in USDC (SPL) or SOL
  (`--token SOL`); both verified against real devnet payments.
- **Correct by construction** — integer-only money math, exact-mint check
  (anti fake-USDC), exact-amount check, reference-based replay resistance,
  `confirmed`/`finalized` gating.
- **Resilient** — RPC failover with bounded, jittered retries; an unreachable
  node keeps an invoice *pending*, never falsely *paid* or *failed*.
- **Reproducible** — pinned toolchain, committed lockfile, one `make` surface,
  115 tests, all offline and deterministic except the opt-in live-network test.
- **Validated agent layer** — the ZeroClaw configuration, skills, and SOPs load
  and pass against a real ZeroClaw v0.8.3 install (`zeroclaw doctor`,
  `skills audit`, `sop validate`), not just documentation-inferred.
- **Human-readable and machine-readable output** — every `solpay` command
  supports `--format json` (default, stable schema) or `--format human`.
- **Dual-licensed** (MIT OR Apache-2.0), with ADRs documenting every major
  design decision.

---

## 7. Technical Documentation

### 7.1 Workspace and crate layout

Single Cargo workspace, one member crate:

```toml
[workspace]
resolver = "2"
members = ["crates/solpay"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/asroryandesfar-art/zeroclaw-solana-pay"
rust-version = "1.96"

[profile.release]
overflow-checks = true   # an arithmetic overflow on funds must panic loudly
panic = "abort"
```

`crates/solpay` builds both a library (`solpay`) and a binary (`solpay`), so the
core logic is unit-testable independent of the CLI surface.

### 7.2 Dependencies (production)

| Crate | Purpose |
|---|---|
| `clap` (derive) | CLI argument parsing |
| `solana-pubkey` (`curve25519`, `sha2` features) | canonical pubkey math — on-curve checks, ATA derivation. No hand-rolled crypto. |
| `serde` / `serde_json` | typed (de)serialization; stable JSON output |
| `ureq` | small blocking HTTP client for JSON-RPC |
| `qrcode` + `image` (png-only) | QR matrix generation and PNG rasterization |
| `percent-encoding` | Solana Pay URL construction |
| `getrandom` | secure randomness for payment references |

Dev-only: `bs58` (test fixtures), `serde_json`.

### 7.3 Code quality gates (all verified in CI)

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings.
- `cargo test --workspace` — **115 tests pass** (+1 real-network test gated
  behind `--ignored`, run via `make test-live`).
- Zero `panic!` / `unwrap()` / `expect()` in production code (test-only).
- No `unsafe` code in `solpay`.
- No unused dependencies; no dead code or duplicated logic.
- Clean release build from a fresh clone; `cargo install --locked` succeeds.

### 7.4 Test composition

| Kind | What it proves |
|---|---|
| Unit tests | pure logic: money math, state machine, validation, URL building, ATA/reference derivation, RPC parsing, verify decision logic |
| CLI black-box tests | the compiled binary's exit codes and JSON contracts, invoked as a subprocess |
| Real-devnet fixture tests | RPC response *parsing* locked against a transaction fetched from live Solana devnet (slot `479289577`) |
| Parse → decide fixture tests | template-substituted real transaction shapes fed through the verifier for paid / underpaid / wrong-mint / missing-reference / overpaid cases |
| Live-network test (`--ignored`) | end-to-end against `api.devnet.solana.com`: create → fresh reference → verify pending |

### 7.5 The ZeroClaw agent layer

Validated against a real **ZeroClaw v0.8.3** install (`schema_version 3`):

- `agent/config.toml` — one agent (`solpay`), one LLM provider entry scoped to
  intent extraction only, WhatsApp channel, a scoped risk profile
  (`allowed_commands = ["solpay", "set", "."]`, high-risk commands blocked),
  and a skill bundle pointing at `agent/skills/solpay/`.
- **Three skills** (`agent/skills/solpay/*/SKILL.toml`): `create-invoice`,
  `send-qr`, `check-payment` — each a thin shell wrapper around `solpay`
  exposing only the model-safe arguments.
- **Two SOPs** (`agent/sops/*/SOP.toml` + `SOP.md`): `charge` (channel-triggered,
  deterministic) and `verify-payments` (cron-triggered, deterministic).
- Deployment is scripted and self-validating: `scripts/setup.sh` copies the
  layer into a ZeroClaw config directory, sets an absolute `sop.sops_dir`, and
  runs `zeroclaw skills list`, `zeroclaw sop validate`, and `zeroclaw doctor`.

A key implementation detail documented in [ADR 0005](docs/adr/0005-validated-against-zeroclaw.md):
ZeroClaw clears a skill's shell environment before executing its command (only
`PATH`, `HOME`, and locale variables survive). The skills therefore **source**
`~/.zeroclaw/solpay.env` (reachable via the surviving `$HOME`) rather than
relying on ambient environment variables — a bug that was caught by validating
against the real runtime instead of documentation alone.

### 7.6 Architecture Decision Records

| ADR | Decision |
|---|---|
| [0001](docs/adr/0001-non-custodial.md) | Non-custodial: store and use only the merchant's receiving public key |
| [0002](docs/adr/0002-llm-quarantine.md) | Quarantine the LLM to language; all money logic is deterministic Rust in SOP deterministic mode |
| [0003](docs/adr/0003-stateless-rust-helper.md) | Ship one stateless Rust CLI (`solpay`), invoked as a ZeroClaw skill, with model-unreachable locked configuration |
| [0004](docs/adr/0004-memory-as-ledger.md) | Invoice ledger lives in ZeroClaw memory (SQLite); `solpay` stays stateless |
| [0005](docs/adr/0005-validated-against-zeroclaw.md) | Validate the entire agent layer against a real ZeroClaw runtime rather than documentation alone |

---

## 8. API Overview (CLI Interface)

**This project exposes a command-line interface, not a network API.** The CLI
subcommands and their stable JSON contracts are the effective programmatic
interface — this is what the ZeroClaw skills call, and what any external
integration would call.

### 8.1 Commands

| Command | Does | Touches network |
|---|---|---|
| `solpay create-url` | build a reference + Solana Pay URL for an amount | no |
| `solpay render-qr` | rasterize a `solana:` URL to a PNG | no |
| `solpay verify` | decide `paid` / `pending` / `mismatch` from the chain | yes (only this) |

Every command supports `--help` and `--format json` (default, machine-readable)
or `--format human` (friendly text).

### 8.2 `create-url`

```
solpay create-url --amount <AMOUNT> --token <TOKEN> [OPTIONS]

--amount <AMOUNT>        Human amount, e.g. "25" or "0.5"
--token <TOKEN>          Token symbol (must be in the allowlist), e.g. "USDC"
--reference <REFERENCE>  Reuse a specific reference (base58); auto-generated if omitted
--label <LABEL>          Label shown in wallets (defaults to STORE_LABEL)
--message <MESSAGE>      Free-text memo, e.g. "Table 4"
--recipient <RECIPIENT>  Merchant wallet (locked; defaults to MERCHANT_WALLET)
--mint <MINT>            Token mint override (locked; defaults to the per-cluster mint)
--cluster <CLUSTER>      Cluster override (locked; defaults to SOLANA_CLUSTER)
```

Output (`--format json`):

```jsonc
{ "reference", "url", "recipient", "mint", "token", "cluster",
  "amount_base_units", "amount_ui", "label", "message" }
```

### 8.3 `render-qr`

```
solpay render-qr --url <URL> --out <OUT> [--scale <N>] [--quiet-zone <N>]
```

Output:

```jsonc
{ "image_path", "format", "size_bytes", "modules", "pixel_size" }
```

`solpay` rejects any URL that does not start with `solana:` — it can never
rasterize an arbitrary link.

### 8.4 `verify`

```
solpay verify --reference <REF> --amount-base-units <N> [OPTIONS]

--token <TOKEN>                  defaults to the invoice's token
--recipient / --mint / --cluster locked config, same as create-url
--commitment <LEVEL>              confirmed | finalized (processed is rejected)
--rpc / --rpc-fallback            RPC endpoints
--signature-limit <N>             bound the transaction scan (SOL verification)
```

Output:

```jsonc
{ "status": "paid|pending|mismatch", "signature": "…|null",
  "slot": 123|null, "reason": "…|null" }
```

### 8.5 Exit codes — the contract ZeroClaw SOPs branch on

| Code | Meaning | SOP reaction |
|---|---|---|
| `0` | success | read the JSON on stdout |
| `2` | invalid input | reject to the user (bad amount/reference/token) |
| `3` | config error | halt; operator misconfiguration |
| `4` | RPC / transient | keep the invoice **pending**; retry next tick |
| `5` | internal error | alert; leave state untouched |

The JSON schemas are **stable — fields are only ever added**, never removed or
renamed, so downstream SOPs and integrations do not break across patch releases.

---

## 9. Installation Guide

### 9.1 Prerequisites

- **Rust** (stable) — the pinned toolchain in `rust-toolchain.toml` is used
  automatically by `rustup`.
- A **Solana devnet RPC** URL. The public `https://api.devnet.solana.com` works
  for a demo; a provider (e.g. Helius) is recommended to avoid rate limits.
- For the full agent (optional, section 9.3): **ZeroClaw v0.8.3+**
  (https://github.com/zeroclaw-labs/zeroclaw) and a WhatsApp Cloud API app *or*
  Web-mode session.

No Solana CLI and no keypair are required for the `solpay` helper itself — it
never signs.

### 9.2 Build and install `solpay`

```bash
git clone https://github.com/asroryandesfar-art/zeroclaw-solana-pay
cd zeroclaw-solana-pay
cp .env.example .env
# edit .env — set MERCHANT_WALLET to your own base58, on-curve public key

make test        # 115 tests, offline and deterministic
make lint        # clippy -D warnings
make install     # installs `solpay` to ~/.cargo/bin
```

### 9.3 Deploy the ZeroClaw agent (WhatsApp terminal)

```bash
# with ZeroClaw installed and on PATH:
scripts/setup.sh
```

This builds and installs `solpay`, deploys `agent/config.toml`, the skill
bundle, and the SOPs into `~/.zeroclaw`, sets an absolute `sop.sops_dir`, and
self-validates with `zeroclaw skills list`, `zeroclaw sop validate`, and
`zeroclaw doctor`. One-time operator steps (edit `~/.zeroclaw/solpay.env`,
set WhatsApp/LLM secrets via `zeroclaw config set`, set the staff allowlist) are
printed at the end of the script and documented in full in
[`docs/SETUP.md`](docs/SETUP.md).

---

## 10. Quick Start

```bash
git clone https://github.com/asroryandesfar-art/zeroclaw-solana-pay
cd zeroclaw-solana-pay
cp .env.example .env          # a demo MERCHANT_WALLET is pre-filled; replace with your own
make test                     # 115 tests, offline & deterministic
make install                  # puts `solpay` on your PATH

set -a; source .env; set +a

# Create an invoice → note the "url" and "reference" in the JSON output
solpay create-url --amount 25 --token USDC --message "Table 4"

# Render its QR
solpay render-qr --url 'solana:...' --out /tmp/qr.png

# Verify payment (returns paid | pending | mismatch)
scripts/verify.sh <reference>
```

Or run **`scripts/demo.sh`**, which performs create → QR → verify against
devnet automatically. Full walkthrough (including USDC/SOL payment and the
ZeroClaw agent): [`docs/SETUP.md`](docs/SETUP.md) and
[Demo Walkthrough](#15-demo-walkthrough) below.

---

## 11. Security Considerations

Full document: [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md). Guiding
assumption: **every external input is malicious until validated** — the message
text, the LLM output, and the RPC node are all untrusted.

### 11.0 Custody tier: T1 (Build)

Unsigned transactions only. `solpay` constructs a Solana Pay transfer-request
URL; the **payer's own wallet** builds and signs the transfer. The agent
process never holds, generates, or uses a private key. Secrets held: **none**.

### 11.1 The asset, and why the blast radius is small

The asset is the merchant's funds. The system is **non-custodial**: it holds no
private key and never signs a transaction. The worst case if the host is fully
compromised is misleading invoice display or denial of service — **not theft of
funds**. Removing custody removes the highest-severity branch entirely.

### 11.1b Prompt-injection test (tested, not asserted)

Two attacks were run against the real code: (1) injecting a `recipient` field
into the `create_invoice` tool call — dropped, because the command template
has no `{{recipient}}` placeholder to substitute into, verified end-to-end
against the real `solpay` binary; (2) talking the agent into a "refund" — a
no-op, because no signing/transfer code exists anywhere in the codebase. Full
transcript: [`docs/PROMPT_INJECTION_TEST.md`](docs/PROMPT_INJECTION_TEST.md).

### 11.2 Threats and mitigations

| Threat | Mitigation | Residual risk |
|---|---|---|
| Prompt injection ("send to my wallet…") | recipient/mint/cluster are locked config, never from the message; LLM output schema + bounds validated | none for fund routing |
| Fake-USDC (token named "USDC") | verifier checks the exact mint, resolved from a per-cluster table, never user input | none |
| Wrong recipient | payment must land in the merchant's derived ATA | none |
| Underpayment / partial | exact base-unit amount check; short → `mismatch` | none (overpay accepted) |
| Acting on a droppable transaction | commitment must be ≥ `confirmed`; `processed` rejected at config | rare `confirmed` reorg → use `finalized` for large amounts |
| Replay / cross-invoice | unique 32-byte reference per invoice; RPC query is *by* reference | none |
| Double-settle / double-confirm | ledger state guard: settle only while `PENDING` | none |
| RPC lies or is down | verdict only from independently verified facts; unreachable ⇒ pending, never paid/failed | availability only |
| Unauthorized sender | WhatsApp `dm_policy=allowlist` + `peer_groups` staff list, deny-by-default | staff device compromise (out of scope) |
| Command injection via skill args | model supplies only amount/token/message; `solpay` rejects invalid input (exit 2); risk profile blocks high-risk commands | depth from the risk profile, not the template alone |
| Secret leakage | WhatsApp/LLM secrets encrypted at rest in ZeroClaw's config store; `solpay.env` holds only a public key | RPC URL may embed a provider key — treat as secret |
| Amount overflow / crafted balances | integer-only, checked math; balance summation in `u128` | none |
| Host compromise | non-custodial ⇒ cannot move funds; audit log append-only | invoice display / DoS |

### 11.3 Assumptions

1. Host integrity — secrets are protected by OS file permissions; even a fully
   compromised host cannot move funds (no private keys exist).
2. RPC may lie or be unavailable — settlement requires independent confirmation
   of mint, amount, recipient ATA, reference, and commitment.
3. The LLM is untrusted-in / structured-out — it cannot set wallet, mint, RPC,
   or exceed limits.
4. The control plane is local and paired — the ZeroClaw gateway binds loopback
   with pairing; external exposure is a deliberate, documented step.
5. Deny-by-default authorization — an empty allowlist means nobody.

### 11.4 Reporting a vulnerability

See [`SECURITY.md`](SECURITY.md) — please do not open a public issue for
security reports; use GitHub Security Advisories instead.

---

## 12. Known Limitations

These are stated plainly, not hidden:

- **Devnet-verified, not mainnet-hardened.** The money logic is identical for
  both clusters, but going to mainnet requires deliberate operator steps: a
  paid RPC provider (public endpoints rate-limit and prune history),
  `PAYMENT_COMMITMENT=finalized` for high-value invoices, and the
  `ALLOW_MAINNET=true` interlock (off by default).
- **Solana Pay URLs have no cluster field.** A wallet chooses the network it is
  set to; the QR cannot force devnet. This project's devnet demo mitigates this
  by using a token mint that only exists on devnet, making a real mainnet
  charge from that QR impossible by construction — but this is a spec property
  worth knowing, not a bug in this project.
- **SOL payments are not reference-bound.** Wallets observed in testing (e.g.
  Phantom) do not attach the Solana Pay reference to native SOL transfers, so
  SOL invoices are matched by exact lamport amount credited to the merchant
  wallet rather than by reference. USDC (SPL) remains strictly reference-bound.
  Practical implication: use a dedicated, low-traffic merchant wallet for SOL
  invoices to avoid ambiguity, and prefer USDC when strict per-invoice binding
  matters.
- **Public devnet RPC rate limits.** SOL verification scans the merchant
  wallet's recent transactions; on a busy wallet with the free
  `api.devnet.solana.com` endpoint, rapid calls can return HTTP 429. This is
  transient and mitigated by `--signature-limit`, retry, or a paid RPC — but it
  is a real operational edge worth planning for.
- **No refund capability, by design.** The agent holds no keys, so it
  structurally cannot issue refunds or sweeps. This is documented as an
  intentional consequence of the non-custodial design (ADR 0001), not an
  oversight — refunds are a manual, merchant-side action.
- **Wallet-side UX gaps observed during testing.** Solflare does not properly
  support Solana Pay on devnet (rejects a valid QR); some Phantom builds do not
  display a devnet USDC balance because the devnet mint lacks on-chain
  metadata (the transfer itself still succeeds). These are third-party wallet
  behaviors, not defects in this project — documented in the README so
  reviewers are not confused during their own testing.
- **Single-instance ledger.** The invoice ledger is a single SQLite file inside
  ZeroClaw's memory layer. There is no multi-writer or clustering story in this
  version — appropriate for a single WhatsApp terminal, not yet designed for
  horizontal scale-out.
- **ZeroClaw agent layer requires a real ZeroClaw host to run.** The `solpay`
  engine has no such dependency, but the WhatsApp terminal experience requires
  a machine running the ZeroClaw daemon with the agent deployed via
  `scripts/setup.sh`.

---

## 13. Roadmap

The project intentionally ships a complete, narrow slice rather than a broad
partially-working one (YAGNI). The items below are the concrete, documented
next steps already identified in the repository's own release checklist — not
speculative feature ideas.

**Near-term (operator deployment, no code changes needed):**
- Deploy the validated agent layer to a persistent ZeroClaw host
  (`scripts/setup.sh`), set the production `MERCHANT_WALLET`, WhatsApp
  Business credentials, LLM API key, and staff allowlist.
- Run the `charge` and `verify-payments` SOPs against a live WhatsApp Business
  number for real staff use.

**Mainnet hardening (documented in [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md)):**
- Provision a paid Solana RPC provider (public endpoints are devnet-appropriate
  only).
- Set `PAYMENT_COMMITMENT=finalized` for high-value invoices.
- Flip `SOLANA_CLUSTER=mainnet-beta` and `ALLOW_MAINNET=true` deliberately, with
  the mainnet USDC mint and merchant wallet confirmed on screen before first use.

**Not committed / no ETA.** Beyond the above, there is no committed roadmap for
additional tokens, additional channels beyond WhatsApp, or a hosted/multi-tenant
version. The token allowlist mechanism (`TOKEN_ALLOWLIST`, per-cluster mint
table in `config.rs`) is structurally extensible to more SPL tokens, but adding
one is future work, not a current feature.

---

## 14. FAQ

**Does the agent ever hold or touch customer funds?**
No. It stores only the merchant's public receiving key. It never generates,
holds, or uses a private key, and never signs a transaction (ADR 0001).

**What happens if the LLM hallucinates an amount or a wallet?**
It can't reach the chain. The LLM's only output is a JSON intent
(`amount`, `token`, `message`) that is schema- and bounds-validated before any
Solana logic runs; the recipient wallet, token mint, and cluster are locked
configuration the model never sees (ADR 0002).

**What if someone sends a "USDC" that isn't real USDC?**
`verify` checks the transaction's **exact token mint** against a compiled
per-cluster table, not against the token's display name. A look-alike token is
rejected as a mismatch.

**What happens if the RPC node is down or lying?**
The invoice is left `PENDING` and retried on the next scheduler tick. The
system never reports a false `PAID` or `FAILED` from an unreachable node — see
exit code `4` in the [API Overview](#8-api-overview-cli-interface).

**Can a customer pay less than the invoice and have it marked paid?**
No — the verifier requires the amount to meet or exceed the exact expected base
units; underpayment is a `mismatch`, not a partial success.

**Does it support tokens other than USDC and SOL?**
Not in this version. `TOKEN_ALLOWLIST` defaults to `USDC,SOL`; the mint
resolution table currently covers those two. See [Roadmap](#13-roadmap).

**Why WhatsApp and not a generic web widget?**
This bounty track is about extending ZeroClaw; WhatsApp is one of ZeroClaw's
built-in Tier-1 channels, so using it kept the project genuinely
ZeroClaw-native rather than bolting a payment SDK onto a custom frontend.

**Is the ZeroClaw agent configuration actually tested, or just documentation?**
It is validated against a real ZeroClaw v0.8.3 binary: `zeroclaw config
list/doctor`, `zeroclaw skills audit/list`, and `zeroclaw sop validate` all
pass on the committed files (ADR 0005). This caught real bugs — for example,
ZeroClaw clears a skill's shell environment, which an earlier,
documentation-only version of the agent config did not account for.

**Is this ready for real money on mainnet today?**
Not without the deliberate steps in [Known Limitations](#12-known-limitations)
and [Roadmap](#13-roadmap) — a paid RPC, `finalized` commitment for large
amounts, and consciously flipping the mainnet interlock. It is fully verified
for devnet and is architecturally mainnet-capable.

**Why Rust for the money core?**
Integer-only, checked arithmetic (`overflow-checks = true`, `panic = "abort"`
in the release profile) and a canonical, audited pubkey/curve library
(`solana-pubkey`) rather than hand-rolled cryptography — appropriate rigor for
code that decides whether money changed hands.

---

## 15. Demo Walkthrough

**Video:** https://youtu.be/t4aPitLXOmo (3 minutes)

The video follows the script in [`docs/DEMO.md`](docs/DEMO.md):

| Time | Content |
|---|---|
| 0:00–0:20 | The pitch: a WhatsApp message becomes a real on-chain payment, run by an agent that never touches the money. |
| 0:20–0:40 | The two-plane architecture: LLM confined to language, every payment decision deterministic. |
| 0:40–1:40 | Live flow: staff messages the agent → QR appears → customer pays 25 USDC on devnet → Solana Explorer shows `confirmed` → agent detects it and replies `Invoice #124 Paid ✅`. |
| 1:40–2:20 | Why it's safe: public-key-only custody, exact-mint check, exact-amount check, one reference per invoice. |
| 2:20–2:50 | `make check`, the CI badge, and the ADR trail. |
| 2:50–3:00 | Repository link. |

**Reproducing the demo yourself (CLI-only, no ZeroClaw required):**

```bash
scripts/demo.sh 1 USDC     # create invoice → QR (/tmp/solpay-demo.png)
# scan the QR with a devnet-funded wallet (e.g. Phantom set to Devnet) and pay
scripts/verify.sh          # → PAID ✅
```

Both USDC and native SOL payment flows have been verified against real devnet
transactions during development — see [`CHANGELOG.md`](CHANGELOG.md) for the
implementation history of each.

---

## 16. Screenshots Section

This project's interface is a **command-line tool plus a WhatsApp
conversation** — there is no graphical dashboard to screenshot. In place of UI
screenshots, this section provides real, reproducible terminal output, and
points to the video for the visual parts (QR code, WhatsApp thread, wallet
approval, Solana Explorer confirmation).

**`solpay create-url` — actual output (devnet, generated while preparing this
submission):**

```json
{"reference":"3Xm36m9eWmYaKC4h6v7ZqmL8ER9DHm3ywsfAFZtBgw9A","url":"solana:9pKSaQGCnfdjFCoHhSAc5mPDDyaBtNeQX2mjFuGNAvmG?amount=25&spl-token=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU&reference=3Xm36m9eWmYaKC4h6v7ZqmL8ER9DHm3ywsfAFZtBgw9A&label=ZeroClaw%20Store","recipient":"9pKSaQGCnfdjFCoHhSAc5mPDDyaBtNeQX2mjFuGNAvmG","mint":"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU","token":"USDC","cluster":"devnet","amount_base_units":25000000,"amount_ui":"25","label":"ZeroClaw Store","message":null}
```

**`solpay verify` on a fresh, unpaid reference — actual output (real devnet
RPC call):**

```json
{"status":"pending","signature":null,"slot":null,"reason":null}
```

**`zeroclaw skills list` against the deployed agent layer — actual output:**

```
Installed skills (3):

  [bundle: solpay]
  send-qr v1.0.0 — Render a Solana Pay URL to a PNG QR code for sending over WhatsApp.
    Tools: render_qr
  create-invoice v1.0.0 — Create a Solana Pay invoice (reference + payment URL) for an amount in an allowed token.
    Tools: create_invoice
  check-payment v1.0.0 — Verify on-chain whether an invoice was paid (paid / pending / mismatch).
    Tools: check_payment
```

**`zeroclaw sop validate` — actual output:**

```
  ✅ charge — valid
  ✅ verify-payments — valid

All SOPs passed validation.
```

For the QR code image, the WhatsApp conversation, the wallet payment approval,
and the Solana Explorer confirmation, see the [demo video](https://youtu.be/t4aPitLXOmo).

---

## 17. Judge Notes

**How to verify this submission quickly (under 10 minutes):**

1. Clone and run `make check` — expect `115 tests` passing, `clippy -D
   warnings` clean, `rustfmt` clean. This is the same command CI runs.
2. Check the CI badge on the repository — green on `main`.
3. Read [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) and
   [`docs/adr/`](docs/adr/) — the security reasoning is written down, not just
   asserted here.
4. Run `scripts/demo.sh 1 USDC` — this creates a real devnet invoice and prints
   a QR; `scripts/verify.sh` will show `pending` until paid. This exercises the
   exact code path used in production, live against Solana devnet.
5. Watch the [demo video](https://youtu.be/t4aPitLXOmo) for the full WhatsApp
   flow ending in a real `PAID` confirmation.

**A concrete, independently checkable piece of on-chain evidence:** the test
suite's real-devnet fixture is drawn from an actual devnet transaction —
signature `67hP2rGMn8snhB9rWc7TJHQrZNHTBvLMAxK78N1dUevWgZhPMxKXt57UxiR2uesFvYMsvaqgGTmiSN6NYCUe7Gsx`,
slot `479289577` — viewable on Solana Explorer (devnet) to confirm this
project's RPC parsing is locked against a real, not synthetic, transaction
shape.

**What to weigh in scoring:**
- The non-custodial and LLM-quarantine decisions are structural, not
  configuration flags — they are enforced by what code exists (no signing
  code path exists at all), not by a setting that could be misconfigured.
- The ZeroClaw agent layer was validated against a real ZeroClaw binary, which
  is unusual rigor for a hackathon submission and caught genuine
  runtime-environment bugs (documented in ADR 0005) that a
  documentation-only implementation would have shipped with.
- The submission is honest about what is **not** yet true: mainnet-readiness,
  refund capability, and multi-token support are explicitly called out as not
  done rather than implied.

**Explicit self-assessment (from [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md)):**
Hackathon-ready — yes. Open-source-ready — yes (dual license, CONTRIBUTING,
SECURITY, CI). Production mainnet-ready — not yet, pending the operator steps
in [Known Limitations](#12-known-limitations).

---

## 18. Appendix — Links & References

- Repository: https://github.com/asroryandesfar-art/zeroclaw-solana-pay
- Release v0.1.0: https://github.com/asroryandesfar-art/zeroclaw-solana-pay/releases/tag/v0.1.0
- Demo video: https://youtu.be/t4aPitLXOmo
- Submission writeup: [`SUBMISSION.md`](SUBMISSION.md)
- Architecture: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Threat model: [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md)
- Prompt-injection test transcript: [`docs/PROMPT_INJECTION_TEST.md`](docs/PROMPT_INJECTION_TEST.md)
- Configuration reference: [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md)
- Setup guide: [`docs/SETUP.md`](docs/SETUP.md)
- Operations runbook: [`docs/OPERATIONS.md`](docs/OPERATIONS.md)
- Demo script: [`docs/DEMO.md`](docs/DEMO.md)
- Architecture Decision Records: [`docs/adr/`](docs/adr/)
- Changelog: [`CHANGELOG.md`](CHANGELOG.md)
- Security policy: [`SECURITY.md`](SECURITY.md)
- ZeroClaw (upstream platform): https://github.com/zeroclaw-labs/zeroclaw
- Solana Pay specification: https://docs.solanapay.com/spec

---

*This document was prepared for hackathon judges and contains only claims that
are backed by the current state of the linked repository at commit `1183475`.
Where a claim depends on an operator action not yet performed (e.g. a mainnet
deployment), it is stated as such rather than implied as complete.*
