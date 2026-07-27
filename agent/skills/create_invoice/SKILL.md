---
name: create_invoice
description: Create an invoice (reference + Solana Pay URL) for a charge.
version: 1.0.0
---

# create_invoice

Use this to turn a validated charge intent into a Solana Pay invoice.

Run:

```
solpay create-url --amount <AMOUNT> --token <TOKEN> --message <MESSAGE>
```

- `AMOUNT` and `TOKEN` come from the parsed intent (e.g. `25`, `USDC`).
- `MESSAGE` is the optional table/memo text.
- **Do not** pass `--recipient`, `--mint`, or `--cluster` — those are fixed by
  the operator's environment and must not be taken from the message.

On success (exit 0) it prints JSON with `reference` and `url`. Store the invoice
in memory as `PENDING`, then hand `url` to `send_qr`.

Exit codes: `0` ok · `2` invalid input (reject to the user) · `3` config error
(halt) · `5` internal.
