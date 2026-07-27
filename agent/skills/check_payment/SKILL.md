---
name: check_payment
description: Verify on-chain whether an invoice has been paid.
version: 1.0.0
---

# check_payment

Check whether a pending invoice has been paid on-chain.

Run:

```
solpay verify --reference <REFERENCE> --amount-base-units <AMOUNT_BASE_UNITS>
```

- Both values come from the stored invoice record.
- **Do not** pass `--recipient`, `--mint`, `--cluster`, `--commitment`, or
  `--rpc` — the verification criteria are fixed by the environment.

Interpret the result strictly by exit code and JSON `status`:

- exit `0`, `status:"paid"` → mark the invoice `PAID` (only if still `PENDING`)
  and send the confirmation once. Record `signature` and `slot`.
- exit `0`, `status:"pending"` → do nothing; check again next tick.
- exit `0`, `status:"mismatch"` → mark `FAILED`, alert staff with `reason`.
- exit `4` → **RPC transient**: leave the invoice `PENDING`, never fail it.
- exit `3`/`5` → operational error; alert, leave state untouched.
