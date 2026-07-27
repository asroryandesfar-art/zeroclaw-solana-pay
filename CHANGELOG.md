# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
