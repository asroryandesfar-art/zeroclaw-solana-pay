//! On-chain *evidence* types — the parsed facts the verifier reasons over.
//!
//! # Why this exists
//! This is the seam of the fetch/decide split. The RPC layer produces these
//! plain structs from raw JSON; the verifier consumes them with no knowledge of
//! HTTP or JSON. Because the verdict is a pure function of these values, every
//! verification case can be tested offline by constructing evidence directly or
//! loading it from a fixture — no live chain required.
//!
//! These types carry only what a payment decision needs. They are intentionally
//! not a general Solana transaction model.

use super::commitment::CommitmentLevel;

/// One entry from `getSignaturesForAddress` for a reference key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureRecord {
    pub signature: String,
    /// `None` when the RPC omitted/!recognized the confirmation status.
    pub confirmation_status: Option<CommitmentLevel>,
    /// True if the transaction failed on-chain (`err != null`).
    pub failed: bool,
}

/// A token-balance change for a single account in one transaction, expressed in
/// base units (integers — never floats).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenBalanceDelta {
    /// Base58 address of the token account whose balance changed.
    pub account: String,
    /// Base58 mint address of the token held in that account.
    pub mint: String,
    pub pre: u64,
    pub post: u64,
}

impl TokenBalanceDelta {
    /// Net increase in base units (0 for a decrease or no change). Saturating so
    /// a malformed decrease can never panic or wrap.
    pub fn increase(&self) -> u64 {
        self.post.saturating_sub(self.pre)
    }
}

/// A native SOL balance change for a single account, in lamports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLamportDelta {
    pub account: String,
    pub pre: u64,
    pub post: u64,
}

impl AccountLamportDelta {
    /// Net lamport increase (0 for a decrease or no change), saturating.
    pub fn increase(&self) -> u64 {
        self.post.saturating_sub(self.pre)
    }
}

/// Everything the verifier needs from one `getTransaction` result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransactionEvidence {
    pub slot: u64,
    /// True if the transaction succeeded on-chain (`meta.err == null`).
    pub succeeded: bool,
    /// Full, resolved account-key list (static message keys followed by any
    /// address-lookup-table loaded writable then readonly keys), in the order
    /// used by `accountIndex` references.
    pub account_keys: Vec<String>,
    pub token_deltas: Vec<TokenBalanceDelta>,
    /// Native SOL lamport deltas per account (from `meta.pre/postBalances`),
    /// used to verify native SOL payments.
    pub lamport_deltas: Vec<AccountLamportDelta>,
}

impl TransactionEvidence {
    /// Whether the given reference key appears among the transaction's accounts.
    pub fn contains_account(&self, address: &str) -> bool {
        self.account_keys.iter().any(|k| k == address)
    }

    /// Net native SOL increase (lamports) credited to `account` in this tx.
    pub fn lamport_increase(&self, account: &str) -> u64 {
        self.lamport_deltas
            .iter()
            .filter(|d| d.account == account)
            .map(|d| d.increase())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increase_is_saturating() {
        let d = TokenBalanceDelta {
            account: "a".into(),
            mint: "m".into(),
            pre: 0,
            post: 25_000_000,
        };
        assert_eq!(d.increase(), 25_000_000);

        // A decrease (outflow) yields 0, never an underflow panic.
        let out = TokenBalanceDelta {
            account: "a".into(),
            mint: "m".into(),
            pre: 10,
            post: 3,
        };
        assert_eq!(out.increase(), 0);
    }

    #[test]
    fn contains_account_matches_exactly() {
        let ev = TransactionEvidence {
            slot: 1,
            succeeded: true,
            account_keys: vec!["ref111".into(), "acc222".into()],
            ..Default::default()
        };
        assert!(ev.contains_account("ref111"));
        assert!(!ev.contains_account("ref"));
        assert!(!ev.contains_account("REF111"));
    }

    #[test]
    fn lamport_increase_sums_credited_accounts() {
        let ev = TransactionEvidence {
            account_keys: vec!["payer".into(), "merchant".into()],
            lamport_deltas: vec![
                AccountLamportDelta {
                    account: "merchant".into(),
                    pre: 0,
                    post: 1_000_000_000,
                },
                AccountLamportDelta {
                    account: "payer".into(),
                    pre: 2_000_000_000,
                    post: 900_000_000,
                },
            ],
            ..Default::default()
        };
        assert_eq!(ev.lamport_increase("merchant"), 1_000_000_000);
        assert_eq!(ev.lamport_increase("payer"), 0); // outflow → 0
    }
}
