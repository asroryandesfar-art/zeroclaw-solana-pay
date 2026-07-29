---
name: create-invoice
description: Create a Solana Pay invoice (reference + payment URL) for an amount in an allowed token.
version: 1.0.0
---

# Create Invoice

Turns a validated charge intent into a Solana Pay invoice.

The `create_invoice` tool runs the stateless `solpay` binary. The model supplies
only `amount`, `token`, and `message`. Every money-bearing value — the merchant
wallet, token mint, cluster, RPC — is loaded from the operator-owned
`~/.zeroclaw/solpay.env` at run time and is **never** model-controlled.

> Why source a file instead of inheriting the environment: ZeroClaw runs skill
> shell commands with a cleared environment (only `PATH`, `HOME`, and a few
> locale vars survive), so the locked config is read from a `$HOME`-relative
> file rather than passed through as ambient env vars.
