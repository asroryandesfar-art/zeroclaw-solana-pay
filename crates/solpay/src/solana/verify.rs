//! Payment verification — the deterministic verdict at the heart of the system.
//!
//! # Why this exists
//! Everything else exists to get us here safely: deciding, from on-chain
//! evidence alone, whether an invoice has been paid. This is the one place a
//! wrong answer costs real money, so it is a pure function — no network, no
//! clock, no floats — and is exhaustively tested against constructed evidence.
//!
//! # The five checks (all must hold to declare `Paid`)
//! 1. **reference present** — the transaction includes this invoice's unique
//!    reference key (so it is *this* invoice's payment, not another's).
//! 2. **exact mint** — funds are the expected USDC mint, not a look-alike token.
//! 3. **correct recipient** — funds landed in the merchant's associated token
//!    account (derived from the merchant wallet + the real mint).
//! 4. **exact amount** — at least the expected base-unit amount arrived.
//! 5. **commitment ≥ required** — the transaction is `confirmed` (or better);
//!    `processed` is never enough.
//!
//! Plus: the transaction must have **succeeded** on-chain (`meta.err == null`).
//!
//! # Threat model
//! The message, the LLM, and the RPC node are all untrusted. None of the five
//! criteria are taken from any of them: `reference`, `mint`, `recipient_ata`,
//! `amount`, and `required_commitment` come from locked config/invoice state.
//! Defended here: fake-USDC (check 2), wrong destination (check 3), underpayment
//! (check 4), acting on droppable transactions (check 5), and replay — a unique
//! reference means a signature for another invoice can neither appear (the RPC
//! query is by reference) nor pass check 1.
//!
//! # Why this design is better
//! Separating the pure verdict from the network fetch means the money decision
//! is fully testable offline and cannot be perturbed by a flaky or hostile node.
//! Summation uses `u128` so crafted balances cannot overflow, and there are no
//! panics on any input.

use super::ata::associated_token_address;
use super::commitment::CommitmentLevel;
use super::model::TransactionEvidence;
use super::rpc::{HttpTransport, RpcClient, RpcError};
use solana_pubkey::Pubkey;

/// The locked expectations an on-chain payment must satisfy. Construct via
/// [`Expected::new`], which derives the merchant's associated token account so
/// callers cannot accidentally pass the wrong destination.
#[derive(Debug, Clone)]
pub struct Expected {
    pub reference: Pubkey,
    pub mint: Pubkey,
    pub recipient_ata: Pubkey,
    pub amount_base_units: u64,
    pub required_commitment: CommitmentLevel,
}

impl Expected {
    pub fn new(
        reference: Pubkey,
        mint: Pubkey,
        merchant_wallet: Pubkey,
        amount_base_units: u64,
        required_commitment: CommitmentLevel,
    ) -> Self {
        let recipient_ata = associated_token_address(&merchant_wallet, &mint);
        Self {
            reference,
            mint,
            recipient_ata,
            amount_base_units,
            required_commitment,
        }
    }
}

/// A signature and its fetched transaction evidence, paired for evaluation.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub signature: String,
    pub commitment: Option<CommitmentLevel>,
    pub evidence: TransactionEvidence,
}

/// The outcome of evaluating a single candidate transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxOutcome {
    Paid,
    NotConfirmed,
    TxFailed,
    MissingReference,
    WrongMint,
    WrongRecipient,
    Underpaid { got: u64 },
}

/// The final verdict for an invoice across all candidate transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Paid { signature: String, slot: u64 },
    Mismatch { reason: String },
    Pending,
}

/// Evaluate one candidate transaction against the locked expectations. Pure.
pub fn evaluate(candidate: &Candidate, expected: &Expected) -> TxOutcome {
    // Check 5 (commitment) first: never reason about content of a transaction
    // that is not yet confirmed enough to act on. Unknown status is conservative.
    if !candidate
        .commitment
        .is_some_and(|c| c.meets(expected.required_commitment))
    {
        return TxOutcome::NotConfirmed;
    }
    // A failed transaction moved no funds.
    if !candidate.evidence.succeeded {
        return TxOutcome::TxFailed;
    }
    // Check 1: this must be *our* invoice's transaction.
    if !candidate
        .evidence
        .contains_account(&expected.reference.to_string())
    {
        return TxOutcome::MissingReference;
    }

    let mint_str = expected.mint.to_string();
    let ata_str = expected.recipient_ata.to_string();

    // Checks 2, 3, 4 via balance deltas. u128 sum so crafted values can't wrap.
    let mut received: u128 = 0;
    for d in &candidate.evidence.token_deltas {
        if d.account == ata_str && d.mint == mint_str {
            received += d.increase() as u128;
        }
    }

    let amount = expected.amount_base_units as u128;
    if received >= amount {
        return TxOutcome::Paid;
    }
    if received > 0 {
        // Correct destination & mint, but not enough arrived.
        return TxOutcome::Underpaid {
            got: received as u64,
        };
    }

    // Nothing valid reached us. Diagnose why, deterministically.
    let right_mint_elsewhere = candidate
        .evidence
        .token_deltas
        .iter()
        .any(|d| d.increase() > 0 && d.mint == mint_str && d.account != ata_str);
    let wrong_mint_inflow = candidate
        .evidence
        .token_deltas
        .iter()
        .any(|d| d.increase() > 0 && d.mint != mint_str);

    if right_mint_elsewhere {
        TxOutcome::WrongRecipient
    } else if wrong_mint_inflow {
        TxOutcome::WrongMint
    } else {
        TxOutcome::WrongRecipient
    }
}

/// Combine per-candidate outcomes into a single verdict.
///
/// Priority: any `Paid` wins (so a duplicate/second valid payment still reads as
/// paid, and the SOP's state guard makes confirmation fire once). Otherwise the
/// most actionable mismatch is reported. Only "not yet" signals (not confirmed,
/// failed, missing reference) leave the invoice `Pending`.
pub fn decide(candidates: &[Candidate], expected: &Expected) -> Verdict {
    let mut underpaid: Option<u64> = None;
    let mut wrong_mint = false;
    let mut wrong_recipient = false;

    for c in candidates {
        match evaluate(c, expected) {
            TxOutcome::Paid => {
                return Verdict::Paid {
                    signature: c.signature.clone(),
                    slot: c.evidence.slot,
                };
            }
            TxOutcome::Underpaid { got } => underpaid = Some(underpaid.map_or(got, |g| g.max(got))),
            TxOutcome::WrongMint => wrong_mint = true,
            TxOutcome::WrongRecipient => wrong_recipient = true,
            TxOutcome::NotConfirmed | TxOutcome::TxFailed | TxOutcome::MissingReference => {}
        }
    }

    if let Some(got) = underpaid {
        return Verdict::Mismatch {
            reason: format!(
                "underpaid: received {got} of {} base units",
                expected.amount_base_units
            ),
        };
    }
    if wrong_mint {
        return Verdict::Mismatch {
            reason: "wrong token mint (not the expected mint)".to_string(),
        };
    }
    if wrong_recipient {
        return Verdict::Mismatch {
            reason: "payment did not reach the merchant token account".to_string(),
        };
    }
    Verdict::Pending
}

/// Orchestrate verification against the chain: find signatures for the
/// reference, fetch each confirmed transaction, and decide. An RPC failure
/// returns `Err` (a transient condition — the caller keeps the invoice PENDING).
pub fn verify_payment<T: HttpTransport>(
    client: &RpcClient<T>,
    expected: &Expected,
    signature_limit: u32,
) -> Result<Verdict, RpcError> {
    let signatures = client.get_signatures_for_address(
        &expected.reference,
        expected.required_commitment,
        signature_limit,
    )?;
    if signatures.is_empty() {
        return Ok(Verdict::Pending);
    }

    let mut candidates = Vec::new();
    for sig in signatures {
        // A failed signature moved no funds; skip the extra fetch.
        if sig.failed {
            continue;
        }
        let evidence = match client.get_transaction(&sig.signature, expected.required_commitment)? {
            Some(ev) => ev,
            None => continue, // dropped between listing and fetch; retry next tick
        };
        candidates.push(Candidate {
            signature: sig.signature,
            commitment: sig.confirmation_status,
            evidence,
        });
    }

    Ok(decide(&candidates, expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana::model::TokenBalanceDelta;

    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const OTHER_MINT: &str = "So11111111111111111111111111111111111111112";
    // A plausible on-curve merchant wallet.
    const MERCHANT: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    fn pk(s: &str) -> Pubkey {
        Pubkey::from_str_const(s)
    }

    fn expected(amount: u64, commitment: CommitmentLevel) -> Expected {
        Expected::new(pk(USDC), pk(USDC), pk(MERCHANT), amount, commitment)
    }

    /// Build evidence: a transfer of `post-pre` of `mint` into `account`, with
    /// `reference` present (unless overridden), succeeded, at `slot`.
    fn evidence_with(
        account: &str,
        mint: &str,
        pre: u64,
        post: u64,
        reference: &str,
        succeeded: bool,
    ) -> TransactionEvidence {
        TransactionEvidence {
            slot: 42,
            succeeded,
            account_keys: vec!["payer".into(), account.to_string(), reference.to_string()],
            token_deltas: vec![TokenBalanceDelta {
                account: account.to_string(),
                mint: mint.to_string(),
                pre,
                post,
            }],
        }
    }

    fn candidate(ev: TransactionEvidence, commitment: CommitmentLevel) -> Candidate {
        Candidate {
            signature: "sig".into(),
            commitment: Some(commitment),
            evidence: ev,
        }
    }

    fn merchant_ata() -> String {
        associated_token_address(&pk(MERCHANT), &pk(USDC)).to_string()
    }

    fn reference_of(exp: &Expected) -> String {
        exp.reference.to_string()
    }

    // ---- valid payment ----
    #[test]
    fn valid_payment_is_paid() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            25_000_000,
            &reference_of(&exp),
            true,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::Paid
        );
    }

    #[test]
    fn overpayment_is_paid() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            30_000_000,
            &reference_of(&exp),
            true,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::Paid
        );
    }

    #[test]
    fn finalized_payment_is_paid() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            25_000_000,
            &reference_of(&exp),
            true,
        );
        // Finalized exceeds the required Confirmed threshold.
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Finalized), &exp),
            TxOutcome::Paid
        );
    }

    // ---- incorrect amount ----
    #[test]
    fn underpayment_is_flagged_with_amount() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            10_000_000,
            &reference_of(&exp),
            true,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::Underpaid { got: 10_000_000 }
        );
    }

    // ---- incorrect mint (fake USDC) ----
    #[test]
    fn wrong_mint_is_rejected() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        // Paid into some account with a different mint; never reaches our ATA.
        let ev = evidence_with(
            "attackerAcct",
            OTHER_MINT,
            0,
            25_000_000,
            &reference_of(&exp),
            true,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::WrongMint
        );
    }

    // ---- incorrect recipient ----
    #[test]
    fn wrong_recipient_is_rejected() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        // Correct mint, but into a different token account.
        let ev = evidence_with(
            "someoneElseAta",
            USDC,
            0,
            25_000_000,
            &reference_of(&exp),
            true,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::WrongRecipient
        );
    }

    // ---- missing reference / replay attempt ----
    #[test]
    fn missing_reference_is_not_paid() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        // A transfer to our ATA of the right amount, but WITHOUT our reference —
        // e.g. an unrelated transaction, or a replay misattributed to us.
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            25_000_000,
            "someOtherReference",
            true,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::MissingReference
        );
    }

    // ---- unconfirmed ----
    #[test]
    fn processed_only_is_not_confirmed() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            25_000_000,
            &reference_of(&exp),
            true,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Processed), &exp),
            TxOutcome::NotConfirmed
        );
    }

    #[test]
    fn unknown_commitment_is_not_confirmed() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            25_000_000,
            &reference_of(&exp),
            true,
        );
        let c = Candidate {
            signature: "s".into(),
            commitment: None,
            evidence: ev,
        };
        assert_eq!(evaluate(&c, &exp), TxOutcome::NotConfirmed);
    }

    // ---- failed tx ----
    #[test]
    fn failed_transaction_moves_no_funds() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            25_000_000,
            &reference_of(&exp),
            false,
        );
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::TxFailed
        );
    }

    // ---- decide(): aggregation ----
    #[test]
    fn decide_pending_when_no_candidates() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        assert_eq!(decide(&[], &exp), Verdict::Pending);
    }

    #[test]
    fn decide_paid_returns_signature_and_slot() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ev = evidence_with(
            &merchant_ata(),
            USDC,
            0,
            25_000_000,
            &reference_of(&exp),
            true,
        );
        let c = Candidate {
            signature: "abc123".into(),
            commitment: Some(CommitmentLevel::Confirmed),
            evidence: ev,
        };
        assert_eq!(
            decide(&[c], &exp),
            Verdict::Paid {
                signature: "abc123".into(),
                slot: 42
            }
        );
    }

    #[test]
    fn decide_duplicate_payments_still_paid_once() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let mk = |sig: &str| Candidate {
            signature: sig.into(),
            commitment: Some(CommitmentLevel::Confirmed),
            evidence: evidence_with(
                &merchant_ata(),
                USDC,
                0,
                25_000_000,
                &reference_of(&exp),
                true,
            ),
        };
        // Two valid payments -> Paid (the SOP guard makes the confirmation fire once).
        match decide(&[mk("first"), mk("second")], &exp) {
            Verdict::Paid { signature, .. } => assert_eq!(signature, "first"),
            other => panic!("expected Paid, got {other:?}"),
        }
    }

    #[test]
    fn decide_multiple_references_one_valid_is_paid() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let bad = Candidate {
            signature: "bad".into(),
            commitment: Some(CommitmentLevel::Confirmed),
            evidence: evidence_with(
                "someoneElseAta",
                USDC,
                0,
                25_000_000,
                &reference_of(&exp),
                true,
            ),
        };
        let good = Candidate {
            signature: "good".into(),
            commitment: Some(CommitmentLevel::Confirmed),
            evidence: evidence_with(
                &merchant_ata(),
                USDC,
                0,
                25_000_000,
                &reference_of(&exp),
                true,
            ),
        };
        match decide(&[bad, good], &exp) {
            Verdict::Paid { signature, .. } => assert_eq!(signature, "good"),
            other => panic!("expected Paid, got {other:?}"),
        }
    }

    #[test]
    fn decide_underpaid_reports_mismatch() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let c = Candidate {
            signature: "s".into(),
            commitment: Some(CommitmentLevel::Confirmed),
            evidence: evidence_with(
                &merchant_ata(),
                USDC,
                0,
                10_000_000,
                &reference_of(&exp),
                true,
            ),
        };
        match decide(&[c], &exp) {
            Verdict::Mismatch { reason } => assert!(reason.contains("underpaid")),
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    #[test]
    fn decide_unconfirmed_stays_pending() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let c = candidate(
            evidence_with(
                &merchant_ata(),
                USDC,
                0,
                25_000_000,
                &reference_of(&exp),
                true,
            ),
            CommitmentLevel::Processed,
        );
        assert_eq!(decide(&[c], &exp), Verdict::Pending);
    }

    // ---- summation across multiple deltas into our ATA ----
    #[test]
    fn multiple_inflows_into_ata_sum_up() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let ata = merchant_ata();
        let mut ev = evidence_with(&ata, USDC, 0, 15_000_000, &reference_of(&exp), true);
        ev.token_deltas.push(TokenBalanceDelta {
            account: ata.clone(),
            mint: USDC.to_string(),
            pre: 15_000_000,
            post: 25_000_000,
        });
        // 15,000,000 + 10,000,000 = 25,000,000 exactly.
        assert_eq!(
            evaluate(&candidate(ev, CommitmentLevel::Confirmed), &exp),
            TxOutcome::Paid
        );
    }

    // ---- verify_payment orchestration (over a scripted transport) ----

    use super::super::rpc::{HttpTransport as _HttpTransport, TransportError};
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::time::Duration;

    struct ScriptedTransport {
        responses: RefCell<VecDeque<Result<String, TransportError>>>,
    }

    impl ScriptedTransport {
        fn new(responses: Vec<Result<String, TransportError>>) -> Self {
            Self {
                responses: RefCell::new(responses.into_iter().collect()),
            }
        }
    }

    impl _HttpTransport for ScriptedTransport {
        fn post_json(&self, _url: &str, _body: &str) -> Result<String, TransportError> {
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err(TransportError::Network("script exhausted".into())))
        }
    }

    fn client(responses: Vec<Result<String, TransportError>>) -> RpcClient<ScriptedTransport> {
        RpcClient::new(
            ScriptedTransport::new(responses),
            vec!["http://mock".into()],
            0,
            Duration::ZERO,
        )
    }

    fn sig_envelope(sig: &str, status: &str) -> String {
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "result": [{ "signature": sig, "confirmationStatus": status, "err": null }]
        })
        .to_string()
    }

    fn tx_envelope(ata: &str, mint: &str, amount: &str, reference: &str) -> String {
        let result = serde_json::json!({
            "slot": 42,
            "meta": {
                "err": null,
                "preTokenBalances": [
                    { "accountIndex": 1, "mint": mint, "uiTokenAmount": { "amount": "0" } }
                ],
                "postTokenBalances": [
                    { "accountIndex": 1, "mint": mint, "uiTokenAmount": { "amount": amount } }
                ]
            },
            "transaction": { "message": { "accountKeys": [
                { "pubkey": "payer" }, { "pubkey": ata }, { "pubkey": reference }
            ] } }
        });
        serde_json::json!({ "jsonrpc": "2.0", "id": 1, "result": result }).to_string()
    }

    #[test]
    fn verify_payment_happy_path_is_paid() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let reference = reference_of(&exp);
        let c = client(vec![
            Ok(sig_envelope("sigPaid", "confirmed")),
            Ok(tx_envelope(&merchant_ata(), USDC, "25000000", &reference)),
        ]);
        let verdict = verify_payment(&c, &exp, 10).unwrap();
        assert_eq!(
            verdict,
            Verdict::Paid {
                signature: "sigPaid".into(),
                slot: 42
            }
        );
    }

    #[test]
    fn verify_payment_no_signatures_is_pending() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let c = client(vec![Ok(serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "result": []
        })
        .to_string())]);
        assert_eq!(verify_payment(&c, &exp, 10).unwrap(), Verdict::Pending);
    }

    #[test]
    fn verify_payment_rpc_failure_propagates() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        // Every attempt times out -> Unavailable (caller keeps invoice PENDING).
        let c = client(vec![Err(TransportError::Timeout)]);
        assert_eq!(verify_payment(&c, &exp, 10), Err(RpcError::Unavailable));
    }

    #[test]
    fn verify_payment_underpaid_is_mismatch() {
        let exp = expected(25_000_000, CommitmentLevel::Confirmed);
        let reference = reference_of(&exp);
        let c = client(vec![
            Ok(sig_envelope("sigShort", "confirmed")),
            Ok(tx_envelope(&merchant_ata(), USDC, "10000000", &reference)),
        ]);
        match verify_payment(&c, &exp, 10).unwrap() {
            Verdict::Mismatch { reason } => assert!(reason.contains("underpaid")),
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }
}
