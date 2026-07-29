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
| the host + filesystem holding `~/.zeroclaw/solpay.env` | WhatsApp message content |
| `agent/config.toml` + encrypted secret store (operator-owned) | LLM output (schema + bounds checked) |
| whoever controls the staff allowlist (`peer_groups`) | RPC responses (re-verified on-chain) |
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
| **Unauthorized sender** | WhatsApp channel `dm_policy = "allowlist"` + `peer_groups` staff list, deny-by-default | staff device compromise (out of scope) |
| **Command injection via skill args** | model supplies only `amount`/`token`/`message`; `solpay` rejects any non-numeric amount / non-allowlisted token (exit 2); the `solpay` risk profile blocks high-risk commands and scopes `allowed_commands` to `solpay` | a crafted arg is still string-substituted into the shell — depth from the risk profile, not the template |
| **Secret leakage** | WhatsApp/LLM secrets are **encrypted at rest** in ZeroClaw's config dir (`[secrets] encrypt = true` + `.secret_key`); `solpay.env` holds only a **public** key and public params; **no private keys exist** | RPC API key in URL — treat that URL as secret |
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

## Prompt-injection test

Custody tier **T1 (Build)** — unsigned transactions, no keys held. See
[`PROMPT_INJECTION_TEST.md`](PROMPT_INJECTION_TEST.md) for two tested attacks
(injecting a recipient into the charge flow; talking the agent into a
"refund") run against the real code, not just asserted.

## Reporting a vulnerability

See [`../SECURITY.md`](../SECURITY.md). Please do not open a public issue for
security reports.
