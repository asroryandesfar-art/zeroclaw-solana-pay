# ADR 0003 — A stateless Rust CLI, invoked as a ZeroClaw skill

**Status:** Accepted

## Context

The deterministic money logic (Pay URL, QR, verification) has to live somewhere
and be callable from ZeroClaw. Options ranged from a compiled ZeroClaw plugin, to
a long-running sidecar service, to a small command-line helper.

## Decision

Ship one **stateless Rust binary** (`solpay`) with three subcommands
(`create-url`, `render-qr`, `verify`), invoked by ZeroClaw `SKILL.toml`
`[[tools]]` shell tools. The dangerous inputs (recipient/mint/cluster/
commitment/rpc) are kept away from the model by **three layers**, not by a
`locked_args` field (which ZeroClaw supports only for `builtin`/`mcp` tools, not
`shell`):

1. The command template omits those flags and exposes only
   `{{amount}}`/`{{token}}`/`{{message}}` as model-supplied args.
2. `solpay` reads the locked values from `~/.zeroclaw/solpay.env`, which the skill
   sources at run time (ZeroClaw clears the environment before running a skill —
   see [ADR 0005](0005-validated-against-zeroclaw.md)).
3. The `solpay` agent risk profile restricts `allowed_commands` to `solpay`
   (plus the `set`/`.` builtins) with high-risk commands blocked.

It owns no database — the invoice ledger lives in ZeroClaw memory — and no keys.

## Consequences

- **+** True Tier-1: no compiled ZeroClaw plugin; capability via skills + a CLI.
- **+** Stateless ⇒ trivially testable, reproducible, and horizontally safe.
- **+** Locked config is unreachable from the model without relying on a ZeroClaw
  feature that does not apply to shell tools.
- **+** One language (Rust) for the whole helper; canonical `solana-pubkey` for crypto.
- **−** A per-invocation process (negligible cost at this scale).

## Alternatives considered

- **Sidecar HTTP service** — rejected for v1: another process to run, document,
  and keep alive, with no benefit at current scale.
- **Compiled ZeroClaw plugin** — rejected: more coupling, less portable, not Tier-1.
- **Hand-rolled crypto** — rejected: ATA/on-curve math uses the canonical crate.
