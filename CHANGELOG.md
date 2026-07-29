# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **ZeroClaw agent layer validated against a real ZeroClaw runtime** (v0.8.3,
  `schema_version 3`). The config, skills, and SOPs now load and pass
  `zeroclaw skills list`, `zeroclaw sop validate`, and `zeroclaw doctor`; the
  `[verify]` caveats are removed. `scripts/setup.sh` deploys the layer into a
  config dir and self-validates. See [ADR 0005](docs/adr/0005-validated-against-zeroclaw.md).
- `agent/solpay.env.example` — the locked money-path config the skills source.

### Changed
- Rewrote the agent layer to the real ZeroClaw schema: `agent/zeroclaw.toml` →
  `agent/config.toml`; skills to `[skill]` + `[[tools]]` (`SKILL.toml`); SOPs to
  per-directory `SOP.toml` + `SOP.md`. WhatsApp/LLM secrets now use ZeroClaw's
  encrypted-at-rest store (`zeroclaw config set`), not `*_env` references.

### Fixed
- **Skill environment.** ZeroClaw clears a skill's environment before running its
  shell command (only `PATH`/`HOME`/locale survive), so `solpay` could not read
  its locked config from ambient env vars. Skills now source
  `~/.zeroclaw/solpay.env` (via the surviving `$HOME`); proven end-to-end under a
  simulated `env_clear`.
- Payment `reference` is now always **on-curve**. It was random 32 bytes (often
  off-curve); strict wallets like **Solflare** reject an off-curve reference as an
  "invalid address" (Phantom is lenient). Now compatible with Phantom and Solflare.

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
- 115 tests: unit, CLI integration (black-box), real-devnet parser fixtures, and
  parse→decide fixtures. CI runs fmt + clippy (`-D warnings`) + tests.
- Documentation: README, architecture, threat model, configuration, setup,
  operations, and ADRs 0001–0005.

### Notes
- Non-custodial by design: no private keys, no signing, no fund custody.
