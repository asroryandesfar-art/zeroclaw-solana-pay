//! Public-key parsing and merchant-wallet validation.
//!
//! We re-use the canonical `solana-pubkey` crate for base58 decoding, the
//! on-curve check, and program-derived-address math — never hand-rolled crypto.

use solana_pubkey::Pubkey;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub enum PubkeyError {
    Invalid {
        input: String,
    },
    /// A merchant wallet must be a normal (on-curve) account. An off-curve key
    /// is a program-derived address / token account and would misroute funds.
    NotOnCurve {
        input: String,
    },
}

impl fmt::Display for PubkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PubkeyError::Invalid { input } => write!(f, "invalid base58 public key: {input}"),
            PubkeyError::NotOnCurve { input } => write!(
                f,
                "public key {input} is not on-curve; a merchant wallet must be a system account, not a token account or PDA"
            ),
        }
    }
}

impl Error for PubkeyError {}

/// Parse any base58 public key (32 bytes). Does not assert on-curve.
pub fn parse_pubkey(s: &str) -> Result<Pubkey, PubkeyError> {
    Pubkey::from_str(s.trim()).map_err(|_| PubkeyError::Invalid {
        input: s.to_string(),
    })
}

/// Parse a merchant receiving wallet and assert it is on-curve. This catches
/// the common, fund-losing mistake of pasting an associated token account or a
/// PDA where a wallet is expected.
pub fn parse_merchant_wallet(s: &str) -> Result<Pubkey, PubkeyError> {
    let pk = parse_pubkey(s)?;
    if pk.is_on_curve() {
        Ok(pk)
    } else {
        Err(PubkeyError::NotOnCurve {
            input: s.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A known on-curve wallet (the SPL Token program owner example wallet is not
    // stable; we use a plain generated on-curve key's string form is unstable,
    // so use the well-known USDC mint which *is* an on-curve mint account).
    const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    #[test]
    fn parses_valid_base58() {
        assert!(parse_pubkey(USDC_MINT).is_ok());
        assert!(parse_pubkey("  EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v  ").is_ok());
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(
            parse_pubkey("not-a-key"),
            Err(PubkeyError::Invalid { .. })
        ));
        assert!(matches!(parse_pubkey(""), Err(PubkeyError::Invalid { .. })));
        // 0 and O and I and l are not in the base58 alphabet.
        assert!(matches!(
            parse_pubkey("0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl"),
            Err(PubkeyError::Invalid { .. })
        ));
    }

    #[test]
    fn merchant_wallet_must_be_on_curve() {
        // The USDC mint is an on-curve account, so it passes the on-curve gate
        // (this test asserts the gate accepts on-curve keys, not that a mint is
        // a good merchant wallet).
        assert!(parse_merchant_wallet(USDC_MINT).is_ok());
    }
}
