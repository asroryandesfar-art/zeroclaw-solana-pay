# Release checklist

Status legend: ✅ done & verified · ⬜ manual step for the operator before going live.

## Code & quality gates (verified)

- ✅ No `TODO` / `FIXME` / `HACK` / `XXX` in tracked files
- ✅ No `panic!` / `unwrap()` / `expect()` in production code (tests only)
- ✅ `cargo fmt --check` clean
- ✅ `cargo clippy --workspace --all-targets -- -D warnings` clean
- ✅ `cargo test --workspace` — 107 tests pass (+1 live-devnet, `--ignored`)
- ✅ Clean release build from scratch (`cargo clean && cargo build --release`)
- ✅ Clean `--locked` install to an isolated root runs (`solpay --version`)

## Repository hygiene (verified)

- ✅ No secrets committed; `.env` git-ignored; only `.env.example` tracked
- ✅ No build artifacts / editor / binary junk tracked (72 files)
- ✅ All regular dependencies are used in `src/`; test-only deps in `dev-dependencies`
- ✅ `Cargo.lock` committed (reproducible builds); toolchain pinned
- ✅ Dual MIT/Apache-2.0 licenses present; CI (fmt + clippy + test + build)
- ✅ README quickstart is copy-paste correct for a first-time developer

## Engine validated on real devnet (verified)

- ✅ `create-url` → valid Solana Pay URL + reference
- ✅ `render-qr` → valid PNG
- ✅ `verify` → `pending` against live `api.devnet.solana.com`
- ✅ `verify` decision paths (`paid` / `mismatch` / underpaid / wrong-mint /
  missing-reference / overpaid) proven via unit tests + a fixture built from a
  **real** devnet USDC transaction shape
- ✅ Exit-code contract (0/2/3/4/5) proven by black-box CLI tests

## Before going live — operator steps (manual)

- ⬜ **Observe a real PAID cycle**: pay a generated QR from a funded devnet USDC
  wallet, then confirm `verify` flips `pending → paid` (this is the money shot
  for the 3-minute video — see `docs/DEMO.md`)
- ⬜ **Validate the ZeroClaw agent layer** on a host with ZeroClaw installed:
  `zeroclaw doctor && zeroclaw channel doctor`; confirm the `[verify]`-marked
  config/SOP/skill formats match your ZeroClaw version
- ⬜ Fill credentials in `.env`: `WHATSAPP_TOKEN`, `WHATSAPP_VERIFY_TOKEN`,
  `WHATSAPP_PHONE_NUMBER_ID`, `LLM_API_KEY`
- ⬜ Set `allowed_users` in `agent/zeroclaw.toml` to the staff phone numbers
- ⬜ Run `scripts/verify_env.sh` → all green
- ⬜ Record the 3-minute demo (`docs/DEMO.md`)

## Mainnet cautions (only when truly ready)

- ⬜ Use a **paid RPC** provider (public endpoints rate-limit)
- ⬜ Consider `PAYMENT_COMMITMENT=finalized` for high-value invoices
- ⬜ Flip the interlock deliberately: `SOLANA_CLUSTER=mainnet-beta` **and**
  `ALLOW_MAINNET=true`
- ⬜ Confirm the mainnet USDC mint and `MERCHANT_WALLET` on screen before first use
- ⬜ Set `chmod 600 .env`; ensure the memory ledger is excluded from consolidation

## Publish

- ⬜ Tag the release (e.g. `v0.1.0`) and update `CHANGELOG.md`
- ⬜ Create the public GitHub repo and push (`master`)
- ⬜ Attach/link the demo video in the README and the bounty submission
