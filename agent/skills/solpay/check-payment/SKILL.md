---
name: check-payment
description: Verify on-chain whether an invoice was paid (paid / pending / mismatch).
version: 1.0.0
---

# Check Payment

The only skill that touches the network. The `check_payment` tool verifies a
payment on-chain. All verification criteria — merchant wallet, mint, cluster,
commitment, RPC — are locked from `~/.zeroclaw/solpay.env`; the model supplies
only the invoice `reference`, expected `amount_base_units`, and `token`.

`solpay` exit code 4 (transient RPC error) is surfaced as "unknown": the caller
must keep the invoice **PENDING**, never flip it to a false PAID/FAILED.
