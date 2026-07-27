//! Commitment levels — the safety ordering for how confirmed a transaction is.
//!
//! # Why this exists
//! Solana transactions progress `processed → confirmed → finalized`. Acting on
//! a `processed` transaction is unsafe: it can still be dropped during a fork.
//! This type gives a single, ordered, misuse-resistant representation shared by
//! config, the RPC layer, and the verifier, so the "how confirmed is enough?"
//! rule is expressed in exactly one place.
//!
//! # Design
//! The enum variants are declared weakest-first so the derived `Ord` yields
//! `Processed < Confirmed < Finalized`. "Is this good enough?" is then a plain
//! `>=` comparison ([`CommitmentLevel::meets`]) — no bespoke comparison logic to
//! get wrong.
//!
//! # Trade-offs
//! We keep `Processed` as a representable value because the chain can *report*
//! it (a signature may be at `processed`); rejecting it as an acceptable
//! *threshold* is a policy decision made by config, not by this type.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommitmentLevel {
    Processed,
    Confirmed,
    Finalized,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseCommitmentError(pub String);

impl fmt::Display for ParseCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown commitment level: {}", self.0)
    }
}

impl Error for ParseCommitmentError {}

impl CommitmentLevel {
    /// Canonical lowercase name as used by the Solana JSON-RPC API.
    pub fn as_str(self) -> &'static str {
        match self {
            CommitmentLevel::Processed => "processed",
            CommitmentLevel::Confirmed => "confirmed",
            CommitmentLevel::Finalized => "finalized",
        }
    }

    /// Parse a commitment level from an RPC/config string. Case-insensitive.
    /// Unknown values are rejected rather than silently defaulted.
    pub fn parse(s: &str) -> Result<Self, ParseCommitmentError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "processed" => Ok(CommitmentLevel::Processed),
            "confirmed" => Ok(CommitmentLevel::Confirmed),
            "finalized" => Ok(CommitmentLevel::Finalized),
            other => Err(ParseCommitmentError(other.to_string())),
        }
    }

    /// True if `self` is at least as confirmed as `required`.
    pub fn meets(self, required: CommitmentLevel) -> bool {
        self >= required
    }
}

impl fmt::Display for CommitmentLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::CommitmentLevel::*;
    use super::*;

    #[test]
    fn ordering_is_weakest_first() {
        assert!(Processed < Confirmed);
        assert!(Confirmed < Finalized);
        assert!(Processed < Finalized);
    }

    #[test]
    fn meets_threshold() {
        assert!(Confirmed.meets(Confirmed));
        assert!(Finalized.meets(Confirmed));
        assert!(!Processed.meets(Confirmed));
        assert!(!Confirmed.meets(Finalized));
    }

    #[test]
    fn parses_case_insensitively() {
        assert_eq!(CommitmentLevel::parse("confirmed"), Ok(Confirmed));
        assert_eq!(CommitmentLevel::parse("  FINALIZED "), Ok(Finalized));
        assert_eq!(CommitmentLevel::parse("Processed"), Ok(Processed));
    }

    #[test]
    fn rejects_unknown() {
        assert_eq!(
            CommitmentLevel::parse("maybe"),
            Err(ParseCommitmentError("maybe".to_string()))
        );
    }

    #[test]
    fn round_trips_as_str() {
        for c in [Processed, Confirmed, Finalized] {
            assert_eq!(CommitmentLevel::parse(c.as_str()), Ok(c));
        }
    }
}
