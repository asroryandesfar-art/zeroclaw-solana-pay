//! `solpay` — the deterministic, stateless Solana Pay helper behind the
//! ZeroClaw Solana Payment Assistant.
//!
//! Design invariants (see `docs/adr`):
//!   * **Non-custodial**: this crate never holds a private key and never signs.
//!     It only formats URLs, renders QR, and *reads* the chain to verify.
//!   * **Fetch/decide split**: network I/O lives in `solana::rpc`; the payment
//!     verdict in `solana::verify` is a pure function of fetched data, so it is
//!     testable offline against fixtures.
//!   * **Integer-only money**: see [`money`]. Floats never touch funds.
//!
//! Modules land incrementally; the public surface grows as each is implemented.

pub mod cli;
pub mod config;
pub mod domain;
pub mod error;
pub mod money;
pub mod output;
pub mod qr;
pub mod solana;
