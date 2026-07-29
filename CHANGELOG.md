# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed
- Native **SOL** payments now verify. Wallets (e.g. Phantom) do **not** attach the
  Solana Pay `reference` to native SOL transfers, so SOL invoices are matched by
  the exact lamport amount credited to the **merchant wallet** (USDC stays
  reference-bound). Proven against a real devnet SOL payment. Note: for a busy or
  shared merchant wallet, use a small `--signature-limit` or a dedicated wallet.

## [0.2.0-dev]

### Added
- Native **SOL** payments alongside USDC. `create-url --token SOL` builds a
  Solana Pay URL with no `spl-token` (native transfer, 9 decimals); `verify
  --token SOL` checks the lamport delta credited to the merchant wallet. Verified
  against a real devnet SOL payment. `TOKEN_ALLOWLIST` now defaults to `USDC,SOL`.

### Changed
- `create-url` JSON `mint` is `null` for native SOL invoices.
- `scripts/demo.sh <amount> [token]` and `scripts/verify.sh` accept the token
  (USDC or SOL); the last invoice's token is remembered.

## [0.1.0] — 2026-07-28

### Added
- `solpay` helper: `create-url`, `render-qr`, and `verify` commands with a stable
  JSON output schema and deterministic exit codes (0/2/3/4/5).
- Integer-only money math; invoice state machine; intent validation.
- Solana layer: base58/on-curve key handling, ATA derivation, unique payment
  references, Solana Pay URL builder, resilient JSON-RPC client (failover +
  bounded retries with jitter), and a pure payment verifier (five checks).
- Configuration with fail-fast validation, a devnet/mainnet master switch, and a
  mainnet safety interlock.
- 107 tests: unit, CLI integration (black-box), real-devnet parser fixtures, and
  parse→decide fixtures. CI runs fmt + clippy (`-D warnings`) + tests.
- Documentation: README, architecture, threat model, configuration, setup,
  operations, and ADRs 0001–0004.

### Notes
- Non-custodial by design: no private keys, no signing, no fund custody.
