//! Money arithmetic. Integer-only — floats never touch funds.
//!
//! Amounts move between two representations:
//!   * a human decimal string ("25", "25.5", "0.01") as typed by a person, and
//!   * `base units` (an integer, e.g. USDC uses 6 decimals so 25 USDC = 25_000_000).
//!
//! Base units are what we verify against on-chain; the decimal string is only
//! ever used to *build* the Solana Pay URL (its `amount` field is a
//! uiAmountString). Parsing is strict and rejects anything ambiguous.

use std::error::Error;
use std::fmt;

/// USDC (and USDC-devnet) use 6 decimals.
pub const USDC_DECIMALS: u8 = 6;

/// Native SOL uses 9 decimals (1 SOL = 1_000_000_000 lamports).
pub const SOL_DECIMALS: u8 = 9;

#[derive(Debug, PartialEq, Eq)]
pub enum MoneyError {
    Empty,
    InvalidChar,
    MultipleDots,
    MissingIntegerPart,
    TooManyDecimals { max: u8 },
    Overflow,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoneyError::Empty => write!(f, "amount is empty"),
            MoneyError::InvalidChar => write!(f, "amount contains a non-digit character"),
            MoneyError::MultipleDots => write!(f, "amount has more than one decimal point"),
            MoneyError::MissingIntegerPart => {
                write!(
                    f,
                    "amount must have a digit before the decimal point (use 0.5, not .5)"
                )
            }
            MoneyError::TooManyDecimals { max } => {
                write!(f, "amount has more than {max} decimal places")
            }
            MoneyError::Overflow => write!(f, "amount is too large"),
        }
    }
}

impl Error for MoneyError {}

/// Parse a human decimal string into integer base units for `decimals`.
///
/// Strictly rejects: empty input, signs, whitespace-in-middle, non-digits,
/// multiple dots, a missing integer part (".5"), more fractional digits than
/// `decimals`, and anything that overflows `u64`. Leading/trailing surrounding
/// whitespace is trimmed. There is deliberately no locale/thousands handling.
pub fn parse_amount(input: &str, decimals: u8) -> Result<u64, MoneyError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(MoneyError::Empty);
    }

    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => {
            if f.contains('.') {
                return Err(MoneyError::MultipleDots);
            }
            (i, f)
        }
        None => (s, ""),
    };

    // Integer part must exist and be all digits.
    if int_part.is_empty() {
        return Err(MoneyError::MissingIntegerPart);
    }
    if !int_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(MoneyError::InvalidChar);
    }
    // Fractional part (may be empty for "25.") must be all digits and fit.
    if !frac_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(MoneyError::InvalidChar);
    }
    if frac_part.len() > decimals as usize {
        return Err(MoneyError::TooManyDecimals { max: decimals });
    }

    // Right-pad the fraction to `decimals` and concatenate, then parse once.
    // Using u128 as the intermediate keeps overflow detection explicit.
    let scale = 10u128
        .checked_pow(decimals as u32)
        .ok_or(MoneyError::Overflow)?;

    let int_val: u128 = int_part.parse().map_err(|_| MoneyError::Overflow)?;
    let frac_val: u128 = if frac_part.is_empty() {
        0
    } else {
        let mut padded = String::with_capacity(decimals as usize);
        padded.push_str(frac_part);
        for _ in 0..(decimals as usize - frac_part.len()) {
            padded.push('0');
        }
        padded.parse().map_err(|_| MoneyError::Overflow)?
    };

    let total = int_val
        .checked_mul(scale)
        .and_then(|v| v.checked_add(frac_val))
        .ok_or(MoneyError::Overflow)?;

    u64::try_from(total).map_err(|_| MoneyError::Overflow)
}

/// Format integer base units back into a minimal decimal string suitable for a
/// Solana Pay `amount` (uiAmountString): no trailing zeros, no trailing dot.
pub fn format_base_units(units: u64, decimals: u8) -> String {
    let scale = 10u64.pow(decimals as u32);
    let whole = units / scale;
    let frac = units % scale;
    if frac == 0 {
        return whole.to_string();
    }
    // Zero-pad the fraction to `decimals`, then strip trailing zeros.
    let frac_str = format!("{frac:0width$}", width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    format!("{whole}.{trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;
    const D: u8 = USDC_DECIMALS;

    #[test]
    fn parses_whole_numbers() {
        assert_eq!(parse_amount("25", D), Ok(25_000_000));
        assert_eq!(parse_amount("0", D), Ok(0));
        assert_eq!(parse_amount("1", D), Ok(1_000_000));
    }

    #[test]
    fn parses_fractions_without_float_drift() {
        assert_eq!(parse_amount("25.5", D), Ok(25_500_000));
        assert_eq!(parse_amount("0.01", D), Ok(10_000));
        assert_eq!(parse_amount("0.000001", D), Ok(1)); // smallest unit
        assert_eq!(parse_amount("0.1", D), Ok(100_000));
        // 0.1 + 0.2 style inputs stay exact because we never use f64.
        assert_eq!(parse_amount("0.30", D), Ok(300_000));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(parse_amount("  25  ", D), Ok(25_000_000));
    }

    #[test]
    fn rejects_too_much_precision() {
        assert_eq!(
            parse_amount("0.0000001", D),
            Err(MoneyError::TooManyDecimals { max: D })
        );
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!(parse_amount("", D), Err(MoneyError::Empty));
        assert_eq!(parse_amount("   ", D), Err(MoneyError::Empty));
        assert_eq!(parse_amount(".5", D), Err(MoneyError::MissingIntegerPart));
        assert_eq!(parse_amount("1.2.3", D), Err(MoneyError::MultipleDots));
        assert_eq!(parse_amount("abc", D), Err(MoneyError::InvalidChar));
        assert_eq!(parse_amount("-5", D), Err(MoneyError::InvalidChar));
        assert_eq!(parse_amount("2 5", D), Err(MoneyError::InvalidChar));
        assert_eq!(parse_amount("25usdc", D), Err(MoneyError::InvalidChar));
    }

    #[test]
    fn rejects_overflow() {
        assert_eq!(parse_amount("99999999999999", D), Err(MoneyError::Overflow));
    }

    #[test]
    fn formats_minimally() {
        assert_eq!(format_base_units(25_000_000, D), "25");
        assert_eq!(format_base_units(500_000, D), "0.5");
        assert_eq!(format_base_units(25_500_000, D), "25.5");
        assert_eq!(format_base_units(1, D), "0.000001");
        assert_eq!(format_base_units(0, D), "0");
        assert_eq!(format_base_units(10_000, D), "0.01");
    }

    #[test]
    fn round_trips() {
        for s in ["0", "25", "0.5", "25.5", "0.01", "0.000001", "1000"] {
            let units = parse_amount(s, D).unwrap();
            assert_eq!(format_base_units(units, D), s, "round-trip failed for {s}");
        }
    }
}
