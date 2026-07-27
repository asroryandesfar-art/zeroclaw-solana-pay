# ADR 0001 — Non-custodial: receiving public key only

**Status:** Accepted

## Context

The agent needs to accept USDC payments on behalf of a merchant. A naive design
would have the service hold a wallet (private key) to receive and manage funds.

## Decision

The system is **non-custodial**. It stores and uses only the merchant's
*receiving public key*. It never holds a private key, never signs, and never
moves funds. Customers pay the merchant's wallet directly via Solana Pay; the
agent only *reads* the chain to verify.

## Consequences

- **+** The highest-severity threat (fund theft) is structurally impossible; a
  fully compromised host cannot move money.
- **+** Onboarding is trivial and safe: paste a public key.
- **+** No secret-key management, rotation, or HSM concerns.
- **−** The agent cannot perform refunds or sweeps; those are manual, merchant-side.

## Alternatives considered

- **Custodial wallet** — rejected: holding keys makes the service a theft target
  and multiplies operational/regulatory risk for no essential benefit.
