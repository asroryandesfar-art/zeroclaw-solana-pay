//! Regression tests that lock our JSON-RPC parsers against the *real* shapes
//! returned by Solana devnet (captured under `tests/fixtures/`). If a future RPC
//! version or a refactor changes how we read `getTransaction` /
//! `getSignaturesForAddress`, these break loudly.

use serde_json::Value;
use solpay::solana::commitment::CommitmentLevel;
use solpay::solana::rpc::{parse_signatures_response, parse_transaction_response};

const USDC_DEVNET_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

#[test]
fn parses_real_devnet_transaction() {
    let raw = include_str!("fixtures/real_devnet_usdc_transfer.json");
    let value: Value = serde_json::from_str(raw).expect("fixture is valid JSON");

    let evidence = parse_transaction_response(&value)
        .expect("real devnet transaction should parse")
        .expect("result is a real transaction, not null");

    // A real, confirmed USDC transfer: succeeded, has a positive slot, resolves
    // account keys, and carries USDC token balances.
    assert!(evidence.succeeded);
    assert!(evidence.slot > 0);
    assert!(!evidence.account_keys.is_empty());

    let usdc_deltas: Vec<_> = evidence
        .token_deltas
        .iter()
        .filter(|d| d.mint == USDC_DEVNET_MINT)
        .collect();
    assert!(
        !usdc_deltas.is_empty(),
        "expected at least one USDC balance entry"
    );

    // Every token account referenced by a balance must be a resolvable account
    // key (index in range) — proven by the fact the delta carries a base58
    // address rather than having failed to parse.
    for d in &evidence.token_deltas {
        assert!(!d.account.is_empty());
    }
}

#[test]
fn parses_real_devnet_signatures() {
    let raw = include_str!("fixtures/real_devnet_signatures.json");
    let value: Value = serde_json::from_str(raw).expect("fixture is valid JSON");

    let records = parse_signatures_response(&value).expect("signatures should parse");
    assert!(!records.is_empty());

    // The captured signatures are finalized and successful.
    for r in &records {
        assert!(!r.signature.is_empty());
        assert!(!r.failed);
        assert_eq!(r.confirmation_status, Some(CommitmentLevel::Finalized));
    }
}
