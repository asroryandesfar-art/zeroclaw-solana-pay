# Release checklist

Legend: ✅ done & verified in this repo · ⬜ operator step before a live/mainnet run.

## Code & quality gates (verified)

- ✅ `cargo fmt --check` clean
- ✅ `cargo clippy --workspace --all-targets -- -D warnings` clean (0 warnings)
- ✅ `cargo test --workspace` — 107 tests pass (+1 live-devnet, `--ignored`)
- ✅ `make check` green
- ✅ Clean release build from scratch; `--locked` install runs
- ✅ **Fresh `git clone` runs the whole README quickstart** (test → install →
  create-url → render-qr → verify) with no hidden steps

## Code audit (verified)

- ✅ No `TODO` / `FIXME` / `HACK` / `XXX` in code
- ✅ No `panic!` / `unwrap()` / `expect()` in production code (tests only)
- ✅ No secrets/credentials committed; `.env` git-ignored (only `.env.example`)
- ✅ Hardcoded values are only canonical SPL program IDs and the per-cluster
  USDC mint table (must be constants); no magic elsewhere
- ✅ No unused dependencies (test-only deps in `dev-dependencies`)
- ✅ No dead code / duplicated logic (single `normalize_symbol`, shared helpers)
- ✅ No data races or leaks: `solpay` is a short-lived, single-threaded,
  stateless CLI with no shared mutable state and no `unsafe`

## Demo end-to-end (verified on real devnet)

- ✅ create invoice → Solana Pay URL (devnet USDC mint)
- ✅ generate QR (PNG)
- ✅ scan + pay — payer's wallet must be on **Devnet** (a Solana Pay URL has no
  cluster field; see `docs/DEMO.md`)
- ✅ verify → `pending` before payment; `paid` after (five on-chain checks)
- ✅ `scripts/verify.sh` auto-loads `.env` (no manual `export`; fixes the
  "MERCHANT_WALLET is not set" foot-gun)
- ⬜ Observe a fresh `pending → paid` on camera (needs a funded devnet USDC
  wallet; run `scripts/demo.sh 25`, pay the QR, then `scripts/verify.sh`)

## Security

- ✅ Non-custodial: no private key, no signing, no custody
- ✅ Mainnet interlock: `ALLOW_MAINNET=true` required for mainnet-beta
- ✅ `processed` rejected as a settlement threshold
- ✅ Exact-mint + exact-recipient + exact-amount + reference + commitment checks
- ✅ RPC transient failures keep an invoice `pending` (never false paid/failed)

## Before submission (operator steps)

- ⬜ Record the 3-minute demo (`docs/DEMO.md`); include a real devnet
  `pending → paid`
- ⬜ Validate the ZeroClaw `agent/` layer on a ZeroClaw host:
  `zeroclaw doctor && zeroclaw channel doctor` (config/SOP/skill are marked
  `[verify]`)
- ⬜ Fill `.env`: `WHATSAPP_TOKEN`, `WHATSAPP_VERIFY_TOKEN`,
  `WHATSAPP_PHONE_NUMBER_ID`, `LLM_API_KEY`
- ⬜ Set `allowed_users` in `agent/zeroclaw.toml` to staff phone numbers
- ⬜ Paste the demo video URL into the README and the submission form

## Mainnet cautions (only when truly ready)

- ⬜ Paid RPC provider (public endpoints rate-limit and prune history)
- ⬜ `PAYMENT_COMMITMENT=finalized` for high-value invoices
- ⬜ `SOLANA_CLUSTER=mainnet-beta` **and** `ALLOW_MAINNET=true`, `chmod 600 .env`
- ⬜ Confirm mainnet USDC mint and `MERCHANT_WALLET` on screen before first use
