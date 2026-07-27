# Operations

Running the assistant in the real world: reliability knobs, the incident
playbook, and the sharp edges worth knowing.

## RPC failover and retries

`verify` tries endpoints in order (`SOLANA_RPC_PRIMARY`, then
`SOLANA_RPC_FALLBACK`), retrying transient failures (timeout, network, HTTP 429
and 5xx) with exponential backoff plus jitter. Non-retryable 4xx moves straight
to the next endpoint. If everything is exhausted, `verify` exits **4** and the
invoice stays **PENDING** — it is retried on the next poll. Configure with
`RPC_TIMEOUT_MS`, `RPC_MAX_RETRIES`, `RPC_BACKOFF_BASE_MS`.

**Recommendation:** use a paid RPC provider for anything beyond a demo; the
public endpoint rate-limits quickly when an invoice's reference is busy.

## Choosing a commitment level

`confirmed` (~1–2s) is the default and is right for typical amounts. For
high-value invoices set `PAYMENT_COMMITMENT=finalized` (~13s) to be fully
reorg-proof. Never use `processed` — it is rejected at config time because a
`processed` transaction can still be dropped in a fork.

## The memory-consolidation gotcha (important)

ZeroClaw's memory layer runs scheduled **consolidation** that summarizes
conversations and marks entries superseded. The invoice ledger must be
**excluded** from consolidation, or financial records could be summarized away.
Keep the `invoices` namespace out of any consolidation policy, and treat
terminal invoices as immutable audit records. (Enforced and tested when the
`agent/` memory layer is wired.)

## Backups

State lives in a single SQLite file (`agent/data/…`). Back it up by copying the
file. It contains invoice records (amounts, staff numbers) — treat it as PII and
never commit it (`agent/data/` is git-ignored).

## Logs

ZeroClaw writes an append-only tool-call audit log (`agent/logs/…`) the agent
itself cannot edit. Use it to reconstruct what happened for any invoice.

## Incident playbook

| Symptom | Likely cause | Action |
|---|---|---|
| Invoices stuck `PENDING` | RPC outage / rate limit | check `SOLANA_RPC_PRIMARY`; add a fallback; confirm exit 4 (not 2/3) |
| `verify` returns `mismatch` | wrong amount / mint / recipient | inspect the `reason`; customer likely underpaid — re-invoice |
| Refuses to start (exit 3) | config error | read stderr; fix the named var (wallet on-curve, https RPC, interlock) |
| "Paid" never sent | confirmation guard already fired, or commitment not reached | check ledger state and commitment level |
| Payment confirmed but reorged | `confirmed` dropped (very rare) | raise to `finalized` for high-value flows |

## Refunds

Out of scope for v1 and, by design, impossible for the agent to perform — it
holds no keys. Refunds are done manually by the merchant from their wallet.

## Monitoring (what to watch)

- backlog of `PENDING` invoices (rising = RPC trouble),
- count of `FAILED` (rising = payment UX or config issue),
- exit-code distribution from `verify` (a spike in 4 = RPC health).
