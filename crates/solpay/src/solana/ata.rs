//! Associated Token Account (ATA) derivation.
//!
//! Solana Pay requires the wallet to send to the recipient's ATA for the given
//! mint. We derive that address exactly the way the SPL Associated Token
//! Account program does — `find_program_address` over
//! `[owner, token_program, mint]` — using the canonical `solana-pubkey` math.
//! The derived address is what `verify` checks the payment landed in.

use solana_pubkey::Pubkey;

/// SPL Associated Token Account program. Parsed at compile time — an invalid
/// literal would fail the build, so there is no runtime panic path.
const ATA_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
/// SPL Token program.
const TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Derive the associated token account for `owner` holding `mint`.
pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (ata, _bump) = Pubkey::find_program_address(
        &[owner.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    );
    ata
}

#[cfg(test)]
mod tests {
    use super::*;

    const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    fn pk(s: &str) -> Pubkey {
        std::str::FromStr::from_str(s).unwrap()
    }

    #[test]
    fn derivation_is_deterministic() {
        let owner = pk(USDC_MINT); // any valid key works as an owner for this test
        let mint = pk(USDC_MINT);
        let a = associated_token_address(&owner, &mint);
        let b = associated_token_address(&owner, &mint);
        assert_eq!(a, b);
    }

    #[test]
    fn ata_is_off_curve() {
        // A program-derived address is, by construction, off the ed25519 curve.
        // This is a real invariant of find_program_address and a good sanity
        // check that we derived a PDA and not something else.
        let owner = pk(USDC_MINT);
        let mint = pk(USDC_MINT);
        let ata = associated_token_address(&owner, &mint);
        assert!(!ata.is_on_curve());
    }

    #[test]
    fn different_owner_or_mint_changes_address() {
        let a = associated_token_address(&pk(USDC_MINT), &pk(USDC_MINT));
        let other = pk("So11111111111111111111111111111111111111112"); // wSOL mint
        let b = associated_token_address(&other, &pk(USDC_MINT));
        assert_ne!(a, b);
    }

    // NOTE: an exact-value regression vector (owner+mint -> known ATA from an
    // explorer) is locked during E2E once a real devnet wallet exists. The
    // derivation itself is canonical (same algorithm as the SPL program).
}
