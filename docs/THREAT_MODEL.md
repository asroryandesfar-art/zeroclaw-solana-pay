# Threat model

The guiding assumption: **every external input is malicious until validated.**
The message text, the LLM output, and the RPC node are all untrusted.

## The asset, and why the blast radius is small

The asset is the merchant's funds. But this system is **non-custodial**: it holds
no private key and never signs a transaction. The worst case if the host is fully
compromised is misleading invoice display or denial of service — **not theft of
funds**. Removing custody removes the highest-severity branch entirely.

## Trust boundaries

| Trusted | Untrusted (validated) |
|---|---|
| the host + filesystem holding `.env` | WhatsApp message content |
| `agent/zeroclaw.toml` (operator-owned) | LLM output (schema + bounds checked) |
| whoever controls `allowed_users` | RPC responses (re-verified on-chain) |
| the compiled `solpay` binary | every customer / payer |

## Threats and mitigations

| Threat | Mitigation | Residual risk |
|---|---|---|
| **Prompt injection** ("send to my wallet…") | recipient/mint/cluster are locked config, never taken from the message; LLM output is schema+bounds validated | none for fund routing |
| **Fake-USDC** (token named "USDC") | verifier checks the **exact mint**; mint resolved from a per-cluster table, not user input | none |
| **Wrong recipient** | payment must land in the merchant's derived ATA | none |
| **Underpayment / partial** | exact base-unit amount check; short → `mismatch` | none (overpay accepted) |
| **Acting on a droppable tx** | require commitment **≥ `confirmed`**; `processed` rejected at config | rare `confirmed` reorg → use `finalized` for large amounts |
| **Replay / cross-invoice** | unique 32-byte `reference` per invoice; the RPC query is *by* reference, and check 1 requires it | none |
| **Double-settle / double-confirm** | ledger state guard: settle only while `PENDING`; re-verify is a no-op | none |
| **RPC lies or is down** | verdict only from independently verified facts; unreachable ⇒ *pending*, never paid/failed | none for correctness (availability only) |
| **Unauthorized sender** | ZeroClaw `allowed_users` deny-by-default allowlist | staff device compromise (out of scope) |
| **Secret leakage** | `.env` git-ignored; config references secrets by env-var name; **no private keys exist** | RPC API key in URL — treat URL as secret |
| **Amount overflow / crafted balances** | integer-only math with checked ops; balance summation in `u128` | none |
| **Host compromise** | non-custodial ⇒ cannot move funds; audit log is append-only | invoice display / DoS |

## Assumptions

1. **Host integrity.** Secrets are protected by OS file permissions. A fully
   compromised host is out of scope — but with no private keys, even that cannot
   move funds.
2. **RPC may lie or be unavailable.** Settlement requires the verifier to
   independently confirm mint, amount, recipient ATA, reference, and commitment.
   "Unavailable" is treated as "unknown."
3. **The LLM is untrusted-in / structured-out.** It cannot set wallet, mint, RPC,
   or exceed limits.
4. **Control plane is local + paired.** The ZeroClaw gateway binds loopback with
   pairing; external exposure is a deliberate, documented step.
5. **Deny-by-default authorization.** An empty allowlist means nobody; onboarding
   staff is an explicit act.

## Reporting a vulnerability

See [`../SECURITY.md`](../SECURITY.md). Please do not open a public issue for
security reports.
