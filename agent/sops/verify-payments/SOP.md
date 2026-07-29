# Verify Payments

Cron-driven settlement loop. Single-flight, deterministic, idempotent: only ever
advances PENDING -> PAID on a proven on-chain match; treats transient RPC failures
(solpay exit 4) as "unknown" and leaves the invoice PENDING.

## Steps

1. **Load pending invoices** — Read all invoices in memory whose status is PENDING; expire any past their deadline.
2. **Verify on-chain** — For each pending invoice call check_payment with its reference, expected amount, and token. Exit code 4 (RPC transient) keeps the invoice PENDING; never mark FAILED on a transient error.
   - tools: check-payment__check_payment
3. **Transition state** — On a PAID verdict atomically move PENDING -> PAID and notify staff. On mismatch keep PENDING.
