//! Intent validation — the deterministic gate between untrusted input and any
//! Solana action. The LLM (or a message) proposes an amount and a token; this
//! module is the sole authority on whether they are acceptable. Wallet, mint,
//! RPC and cluster are never validated here because they are never taken from
//! input — they come from locked config.

use std::error::Error;
use std::fmt;

/// Charge amount bounds, in base units, from `MIN_CHARGE`/`MAX_CHARGE`.
#[derive(Debug, Clone, Copy)]
pub struct ChargeLimits {
    pub min_base_units: u64,
    pub max_base_units: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    ZeroAmount,
    BelowMinimum { min: u64 },
    AboveMaximum { max: u64 },
    TokenNotAllowed { token: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::ZeroAmount => write!(f, "amount must be greater than zero"),
            ValidationError::BelowMinimum { min } => {
                write!(f, "amount is below the minimum charge ({min} base units)")
            }
            ValidationError::AboveMaximum { max } => {
                write!(f, "amount is above the maximum charge ({max} base units)")
            }
            ValidationError::TokenNotAllowed { token } => {
                write!(f, "token '{token}' is not in the allowlist")
            }
        }
    }
}

impl Error for ValidationError {}

/// Validate a charge amount (base units) against configured limits.
pub fn validate_amount(units: u64, limits: &ChargeLimits) -> Result<(), ValidationError> {
    if units == 0 {
        return Err(ValidationError::ZeroAmount);
    }
    if units < limits.min_base_units {
        return Err(ValidationError::BelowMinimum {
            min: limits.min_base_units,
        });
    }
    if units > limits.max_base_units {
        return Err(ValidationError::AboveMaximum {
            max: limits.max_base_units,
        });
    }
    Ok(())
}

/// Normalize a token symbol for comparison (uppercase, trimmed).
pub fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

/// Validate that a token symbol is in the allowlist. Comparison is
/// case-insensitive; the allowlist is expected already normalized upstream but
/// we normalize both sides defensively.
pub fn validate_token(symbol: &str, allowlist: &[String]) -> Result<(), ValidationError> {
    let want = normalize_symbol(symbol);
    if allowlist.iter().any(|a| normalize_symbol(a) == want) {
        Ok(())
    } else {
        Err(ValidationError::TokenNotAllowed { token: want })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ChargeLimits {
        // 0.01 .. 1000 USDC in base units (6 decimals)
        ChargeLimits {
            min_base_units: 10_000,
            max_base_units: 1_000_000_000,
        }
    }

    #[test]
    fn accepts_in_range() {
        assert_eq!(validate_amount(25_000_000, &limits()), Ok(()));
        assert_eq!(validate_amount(10_000, &limits()), Ok(())); // exactly min
        assert_eq!(validate_amount(1_000_000_000, &limits()), Ok(())); // exactly max
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(
            validate_amount(0, &limits()),
            Err(ValidationError::ZeroAmount)
        );
        assert_eq!(
            validate_amount(9_999, &limits()),
            Err(ValidationError::BelowMinimum { min: 10_000 })
        );
        assert_eq!(
            validate_amount(1_000_000_001, &limits()),
            Err(ValidationError::AboveMaximum { max: 1_000_000_000 })
        );
    }

    #[test]
    fn token_allowlist_is_case_insensitive() {
        let allow = vec!["USDC".to_string()];
        assert_eq!(validate_token("USDC", &allow), Ok(()));
        assert_eq!(validate_token("usdc", &allow), Ok(()));
        assert_eq!(validate_token("  Usdc ", &allow), Ok(()));
    }

    #[test]
    fn rejects_token_not_in_allowlist() {
        let allow = vec!["USDC".to_string()];
        assert_eq!(
            validate_token("SOL", &allow),
            Err(ValidationError::TokenNotAllowed {
                token: "SOL".to_string()
            })
        );
        // A look-alike scam token symbol is rejected too; the real defense is
        // the exact-mint check at verify time, but validation stops it early.
        assert_eq!(
            validate_token("USDT", &allow),
            Err(ValidationError::TokenNotAllowed {
                token: "USDT".to_string()
            })
        );
    }
}
