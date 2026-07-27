//! End-to-end decision tests over realistic transaction JSON: a template shaped
//! like a real devnet `getTransaction` result is parameterized per case, then
//! run through the *actual* parser and the *actual* verdict logic. This exercises
//! the full parse -> decide pipeline (not just constructed structs) for the
//! money-critical outcomes.

use serde_json::Value;
use solpay::solana::ata::associated_token_address;
use solpay::solana::commitment::CommitmentLevel;
use solpay::solana::pubkey::parse_pubkey;
use solpay::solana::rpc::parse_transaction_response;
use solpay::solana::verify::{decide, Candidate, Expected, Verdict};

const TEMPLATE: &str = include_str!("fixtures/payment_template.json");

const USDC_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
const MERCHANT: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
const REFERENCE: &str = "So11111111111111111111111111111111111111112";
// Distinct valid pubkeys for the "wrong" cases.
const OTHER_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // mainnet USDC != devnet USDC
const WRONG_REF: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

const FIXTURE_SLOT: u64 = 479_289_577;

fn merchant_ata() -> String {
    let mint = parse_pubkey(USDC_DEVNET).unwrap();
    let merchant = parse_pubkey(MERCHANT).unwrap();
    associated_token_address(&merchant, &mint).to_string()
}

fn expected(amount: u64) -> Expected {
    Expected::new(
        parse_pubkey(REFERENCE).unwrap(),
        parse_pubkey(USDC_DEVNET).unwrap(),
        parse_pubkey(MERCHANT).unwrap(),
        amount,
        CommitmentLevel::Confirmed,
    )
}

/// Instantiate the template into a parsed candidate.
fn candidate(account: &str, mint: &str, reference: &str, pre: u64, post: u64) -> Candidate {
    let json = TEMPLATE
        .replace("__ACCOUNT__", account)
        .replace("__MINT__", mint)
        .replace("__REF__", reference)
        .replace("__OWNER__", MERCHANT)
        .replace("__PRE__", &pre.to_string())
        .replace("__POST__", &post.to_string())
        .replace("__PRE_UI__", "0")
        .replace("__POST_UI__", "0");
    let value: Value = serde_json::from_str(&json).expect("template instantiates to valid JSON");
    let evidence = parse_transaction_response(&value)
        .expect("template parses")
        .expect("template is a transaction, not null");
    Candidate {
        signature: "fixtureSig".to_string(),
        commitment: Some(CommitmentLevel::Confirmed),
        evidence,
    }
}

#[test]
fn valid_payment_fixture_is_paid() {
    let c = candidate(&merchant_ata(), USDC_DEVNET, REFERENCE, 0, 25_000_000);
    assert_eq!(
        decide(&[c], &expected(25_000_000)),
        Verdict::Paid {
            signature: "fixtureSig".to_string(),
            slot: FIXTURE_SLOT
        }
    );
}

#[test]
fn underpaid_fixture_is_mismatch() {
    let c = candidate(&merchant_ata(), USDC_DEVNET, REFERENCE, 0, 10_000_000);
    match decide(&[c], &expected(25_000_000)) {
        Verdict::Mismatch { reason } => assert!(reason.contains("underpaid"), "reason: {reason}"),
        other => panic!("expected Mismatch, got {other:?}"),
    }
}

#[test]
fn wrong_mint_fixture_is_mismatch() {
    // Funds moved with a different mint into a non-merchant account.
    let c = candidate(MERCHANT, OTHER_MINT, REFERENCE, 0, 25_000_000);
    match decide(&[c], &expected(25_000_000)) {
        Verdict::Mismatch { reason } => assert!(reason.contains("mint"), "reason: {reason}"),
        other => panic!("expected Mismatch, got {other:?}"),
    }
}

#[test]
fn missing_reference_fixture_is_pending() {
    // A correct-looking transfer to our ATA, but without our reference.
    let c = candidate(&merchant_ata(), USDC_DEVNET, WRONG_REF, 0, 25_000_000);
    assert_eq!(decide(&[c], &expected(25_000_000)), Verdict::Pending);
}

#[test]
fn overpaid_fixture_is_paid() {
    let c = candidate(&merchant_ata(), USDC_DEVNET, REFERENCE, 0, 30_000_000);
    assert!(matches!(
        decide(&[c], &expected(25_000_000)),
        Verdict::Paid { .. }
    ));
}
