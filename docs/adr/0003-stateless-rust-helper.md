# ADR 0003 — A stateless Rust CLI, invoked as a ZeroClaw skill

**Status:** Accepted

## Context

The deterministic money logic (Pay URL, QR, verification) has to live somewhere
and be callable from ZeroClaw. Options ranged from a compiled ZeroClaw plugin, to
a long-running sidecar service, to a small command-line helper.

## Decision

Ship one **stateless Rust binary** (`solpay`) with three subcommands
(`create-url`, `render-qr`, `verify`), invoked by ZeroClaw `SKILL.toml` shell
tools with **locked args** (recipient/mint/cluster/commitment/rpc). It owns no
database — the invoice ledger lives in ZeroClaw memory — and no keys.

## Consequences

- **+** True Tier-1: no compiled ZeroClaw plugin; capability via skills + a CLI.
- **+** Stateless ⇒ trivially testable, reproducible, and horizontally safe.
- **+** `locked_args` keep the dangerous inputs unreachable from the model.
- **+** One language (Rust) for the whole helper; canonical `solana-pubkey` for crypto.
- **−** A per-invocation process (negligible cost at this scale).

## Alternatives considered

- **Sidecar HTTP service** — rejected for v1: another process to run, document,
  and keep alive, with no benefit at current scale.
- **Compiled ZeroClaw plugin** — rejected: more coupling, less portable, not Tier-1.
- **Hand-rolled crypto** — rejected: ATA/on-curve math uses the canonical crate.
