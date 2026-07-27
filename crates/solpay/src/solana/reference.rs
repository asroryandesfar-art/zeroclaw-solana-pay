//! Payment `reference` generation.
//!
//! Each invoice gets a unique 32-byte `reference` that Solana Pay embeds as a
//! read-only account in the transfer. It is the join key between an invoice and
//! its on-chain transaction, and it is what makes cross-invoice replay
//! impossible: a signature found for reference A can never settle invoice B.
//!
//! The reference is NOT a signing key — we never hold a secret for it — so it
//! need not be on-curve. We use cryptographically secure random bytes.

use solana_pubkey::Pubkey;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ReferenceError(String);

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to generate secure random reference: {}", self.0)
    }
}

impl Error for ReferenceError {}

/// Generate a fresh, unique payment reference from a secure RNG.
pub fn generate() -> Result<Pubkey, ReferenceError> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| ReferenceError(e.to_string()))?;
    Ok(Pubkey::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn references_are_unique() {
        // Not a statistical proof, but a fresh batch must have no collisions.
        let mut seen = HashSet::new();
        for _ in 0..10_000 {
            let r = generate().unwrap();
            assert!(seen.insert(r), "reference collision — RNG is broken");
        }
    }

    #[test]
    fn reference_is_32_bytes_base58() {
        let r = generate().unwrap();
        let s = r.to_string();
        // base58 of 32 bytes decodes back to 32 bytes.
        let decoded = bs58::decode(&s).into_vec().unwrap();
        assert_eq!(decoded.len(), 32);
    }
}
