# ADR 0002 — Quarantine the LLM from money decisions

**Status:** Accepted

## Context

An LLM is excellent at understanding "Charge Table 4 25 USDC" and at writing a
friendly reply. It is a poor and non-deterministic choice for arithmetic on money
or for deciding whether a payment occurred.

## Decision

The LLM does exactly two things: turn a message into a **validated intent JSON**,
and compose reply text. Every money operation — amount parsing, URL building, and
the payment verdict — is deterministic Rust. The ZeroClaw SOP runs the money path
in **deterministic mode** (no LLM round-trips at settlement). The boundary is a
strict JSON schema plus bounds validation; anything failing it is rejected before
any Solana action.

## Consequences

- **+** A hallucinated amount or wallet cannot reach the chain.
- **+** Settlement is reproducible and unit-testable; no model variance on money.
- **+** Prompt injection cannot set recipient/mint/limits — those are locked config.
- **−** Slightly more plumbing (an explicit intent contract) than "let the model do it."

## Alternatives considered

- **LLM decides "is this paid?"** — rejected: non-deterministic decisions on funds
  are unacceptable and untestable.
- **LLM computes amounts/URLs** — rejected: money math must be exact and integer-only.
