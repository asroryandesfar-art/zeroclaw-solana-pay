# ADR 0004 — Invoice ledger in ZeroClaw memory, verifier stays pure

**Status:** Accepted

## Context

Invoices need durable, structured state (status, reference, amount, timestamps),
queryable by status, with idempotent transitions. ZeroClaw provides a SQLite-backed
memory layer; we want to reuse it rather than add infrastructure.

## Decision

Use ZeroClaw memory (SQLite) as the invoice **ledger** in a dedicated `invoices`
namespace — the single source of truth. `solpay` remains **stateless**: it emits
verdicts and artifacts; the SOP writes state. Idempotency comes from a
single-writer model + single-flight cron + a guarded transition (settle only
while `PENDING`), so hardware CAS is not required. The ledger is **excluded from
memory consolidation** so financial records are never summarized away. If the
memory layer proves unable to index status or update in place, the documented
fallback is a dedicated `invoices` table in the *same* SQLite file.

## Consequences

- **+** No new infrastructure; one file to back up; reuses ZeroClaw persistence.
- **+** No duplicated state — `reference` is the single key; QR is re-derivable.
- **+** Confirmation fires exactly once via the transition guard.
- **−** Requires an explicit consolidation-exclusion policy (a real gotcha, tested).

## Alternatives considered

- **Separate database** — rejected unless memory proves insufficient; kept as a
  same-file fallback to avoid new infra.
- **Verifier owns a DB** — rejected: would duplicate state and break statelessness.
