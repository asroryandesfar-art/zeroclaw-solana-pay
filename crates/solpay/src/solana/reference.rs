//! Payment `reference` generation.
//!
//! Each invoice gets a unique 32-byte `reference` that Solana Pay embeds as a
//! read-only account in the transfer. It is the join key between an invoice and
//! its on-chain transaction, and it is what makes cross-invoice replay
//! impossible: a signature found for reference A can never settle invoice B.
//!
//! The reference is NOT a signing key — we never hold a secret for it. It MUST,
//! however, be **on-curve**: the Solana Pay convention is a keypair public key,
//! and strict wallets (e.g. Solflare) reject an off-curve reference as an
//! "invalid address". We generate secure random bytes and keep only on-curve
//! candidates.

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

/// Generate a fresh, unique, **on-curve** payment reference from a secure RNG.
///
/// Roughly half of random 32-byte values decode to a valid ed25519 curve point,
/// so we retry until we get an on-curve public key (a keypair-style reference
/// that every wallet accepts). The bound makes failure astronomically unlikely.
pub fn generate() -> Result<Pubkey, ReferenceError> {
    for _ in 0..128 {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).map_err(|e| ReferenceError(e.to_string()))?;
        let pk = Pubkey::from(bytes);
        if pk.is_on_curve() {
            return Ok(pk);
        }
    }
    Err(ReferenceError("could not generate an on-curve reference".to_string()))
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

    #[test]
    fn references_are_always_on_curve() {
        // Wallets like Solflare reject off-curve references as invalid addresses.
        for _ in 0..2_000 {
            assert!(generate().unwrap().is_on_curve(), "generated an off-curve reference");
        }
    }
}
