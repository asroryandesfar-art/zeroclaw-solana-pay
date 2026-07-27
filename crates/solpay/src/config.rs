//! Configuration resolution and validation.
//!
//! # Why this exists
//! Every dangerous value — merchant wallet, token mint, cluster, commitment,
//! RPC endpoints — must be validated *before* any Solana action, and must come
//! from locked config rather than from a message or the LLM. This module is the
//! single place that turns raw strings (from CLI flags or environment) into
//! validated, typed values, failing fast with a config error code (3) on
//! anything suspicious.
//!
//! # Design
//! The core is a set of **pure functions** over explicit string inputs, so they
//! are tested hermetically with no environment mutation. Thin `*_from_env`
//! wrappers read `std::env` and delegate. This keeps unit tests deterministic
//! and free of global-state flakiness.
//!
//! # Security
//! * A merchant wallet must be **on-curve** (rejects a token account / PDA).
//! * `mainnet-beta` requires an explicit `ALLOW_MAINNET=true` interlock.
//! * `processed` is rejected as a settlement threshold (fork-unsafe).
//! * RPC endpoints must be `https` (or `http` only for localhost).
//! * The token mint is resolved from a compiled per-cluster table, never taken
//!   as free text — the anti-fake-USDC seam.

use std::error::Error;
use std::fmt;

use solana_pubkey::Pubkey;

use crate::domain::validation::{normalize_symbol, ChargeLimits};
use crate::money::{parse_amount, USDC_DECIMALS};
use crate::solana::commitment::CommitmentLevel;
use crate::solana::pubkey::{parse_merchant_wallet, parse_pubkey};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing {
        var: &'static str,
    },
    InvalidWallet {
        detail: String,
    },
    UnknownCluster {
        value: String,
    },
    MainnetNotAllowed,
    ProcessedNotAllowed,
    UnknownCommitment {
        value: String,
    },
    TokenNotAllowed {
        token: String,
    },
    MintUnresolved {
        token: String,
        cluster: &'static str,
    },
    InvalidMint {
        detail: String,
    },
    InvalidRpcUrl {
        url: String,
    },
    InvalidLimit {
        detail: String,
    },
    InvalidNumber {
        var: &'static str,
        value: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Missing { var } => write!(f, "required configuration `{var}` is not set"),
            ConfigError::InvalidWallet { detail } => write!(f, "invalid merchant wallet: {detail}"),
            ConfigError::UnknownCluster { value } => {
                write!(f, "unknown cluster `{value}` (expected `devnet` or `mainnet-beta`)")
            }
            ConfigError::MainnetNotAllowed => write!(
                f,
                "refusing to run on mainnet-beta without ALLOW_MAINNET=true (safety interlock)"
            ),
            ConfigError::ProcessedNotAllowed => write!(
                f,
                "`processed` is not an acceptable commitment threshold; use `confirmed` or `finalized`"
            ),
            ConfigError::UnknownCommitment { value } => {
                write!(f, "unknown commitment `{value}`")
            }
            ConfigError::TokenNotAllowed { token } => {
                write!(f, "token `{token}` is not in the allowlist")
            }
            ConfigError::MintUnresolved { token, cluster } => {
                write!(f, "no known mint for token `{token}` on `{cluster}`")
            }
            ConfigError::InvalidMint { detail } => write!(f, "invalid mint override: {detail}"),
            ConfigError::InvalidRpcUrl { url } => {
                write!(f, "invalid RPC URL `{url}` (must be https, or http only for localhost)")
            }
            ConfigError::InvalidLimit { detail } => write!(f, "invalid charge limits: {detail}"),
            ConfigError::InvalidNumber { var, value } => {
                write!(f, "`{var}` is not a valid number: {value}")
            }
        }
    }
}

impl Error for ConfigError {}

// ---------------------------------------------------------------------------
// Cluster + mint table
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cluster {
    Devnet,
    MainnetBeta,
}

impl Cluster {
    pub fn as_str(self) -> &'static str {
        match self {
            Cluster::Devnet => "devnet",
            Cluster::MainnetBeta => "mainnet-beta",
        }
    }
}

/// Canonical USDC mint per cluster. Resolving the mint from a table (never from
/// user input) is what makes a token merely *named* "USDC" impossible to pass
/// off as the real thing.
const USDC_MINT_MAINNET: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
// Circle's USDC on devnet.
const USDC_MINT_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

fn usdc_mint(cluster: Cluster) -> &'static str {
    match cluster {
        Cluster::MainnetBeta => USDC_MINT_MAINNET,
        Cluster::Devnet => USDC_MINT_DEVNET,
    }
}

// ---------------------------------------------------------------------------
// Pure resolvers (hermetic; no env)
// ---------------------------------------------------------------------------

/// Resolve and gate the cluster. `mainnet-beta` requires `allow_mainnet`.
pub fn resolve_cluster(value: &str, allow_mainnet: bool) -> Result<Cluster, ConfigError> {
    let cluster = match value.trim().to_ascii_lowercase().as_str() {
        "devnet" => Cluster::Devnet,
        "mainnet-beta" | "mainnet" => Cluster::MainnetBeta,
        other => {
            return Err(ConfigError::UnknownCluster {
                value: other.to_string(),
            })
        }
    };
    if cluster == Cluster::MainnetBeta && !allow_mainnet {
        return Err(ConfigError::MainnetNotAllowed);
    }
    Ok(cluster)
}

/// Parse and require an on-curve merchant wallet.
pub fn resolve_recipient(value: &str) -> Result<Pubkey, ConfigError> {
    parse_merchant_wallet(value).map_err(|e| ConfigError::InvalidWallet {
        detail: e.to_string(),
    })
}

/// Reject `processed`; accept `confirmed`/`finalized`.
pub fn resolve_commitment(value: &str) -> Result<CommitmentLevel, ConfigError> {
    match CommitmentLevel::parse(value) {
        Ok(CommitmentLevel::Processed) => Err(ConfigError::ProcessedNotAllowed),
        Ok(level) => Ok(level),
        Err(_) => Err(ConfigError::UnknownCommitment {
            value: value.to_string(),
        }),
    }
}

/// Resolve the token mint. A `mint_override` (locked flag) wins; otherwise the
/// mint is taken from the per-cluster table. The token must be in `allowlist`.
pub fn resolve_mint(
    token: &str,
    cluster: Cluster,
    mint_override: Option<&str>,
    allowlist: &[String],
) -> Result<Pubkey, ConfigError> {
    let want = normalize_symbol(token);
    if !allowlist.iter().any(|a| normalize_symbol(a) == want) {
        return Err(ConfigError::TokenNotAllowed { token: want });
    }
    if let Some(m) = mint_override {
        return parse_pubkey(m).map_err(|e| ConfigError::InvalidMint {
            detail: e.to_string(),
        });
    }
    let mint_str = match want.as_str() {
        "USDC" => usdc_mint(cluster),
        _ => {
            return Err(ConfigError::MintUnresolved {
                token: want,
                cluster: cluster.as_str(),
            })
        }
    };
    parse_pubkey(mint_str).map_err(|e| ConfigError::InvalidMint {
        detail: e.to_string(),
    })
}

/// Validate and order RPC endpoints (primary first). Requires `https`, except
/// `http` is permitted for localhost (local test validator).
pub fn resolve_rpc_endpoints(
    primary: Option<&str>,
    fallback: Option<&str>,
) -> Result<Vec<String>, ConfigError> {
    let primary = primary.ok_or(ConfigError::Missing {
        var: "SOLANA_RPC_PRIMARY",
    })?;
    let mut endpoints = vec![validate_rpc_url(primary)?];
    if let Some(f) = fallback {
        if !f.trim().is_empty() {
            endpoints.push(validate_rpc_url(f)?);
        }
    }
    Ok(endpoints)
}

fn validate_rpc_url(url: &str) -> Result<String, ConfigError> {
    let u = url.trim();
    if u.starts_with("https://") {
        return Ok(u.to_string());
    }
    if u.starts_with("http://") {
        let host = u.trim_start_matches("http://");
        if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
            return Ok(u.to_string());
        }
    }
    Err(ConfigError::InvalidRpcUrl { url: u.to_string() })
}

/// Parse charge limits (decimal strings) into base-unit bounds.
pub fn resolve_charge_limits(min: &str, max: &str) -> Result<ChargeLimits, ConfigError> {
    let min_base = parse_amount(min, USDC_DECIMALS).map_err(|e| ConfigError::InvalidLimit {
        detail: format!("MIN_CHARGE: {e}"),
    })?;
    let max_base = parse_amount(max, USDC_DECIMALS).map_err(|e| ConfigError::InvalidLimit {
        detail: format!("MAX_CHARGE: {e}"),
    })?;
    if min_base == 0 {
        return Err(ConfigError::InvalidLimit {
            detail: "MIN_CHARGE must be > 0".to_string(),
        });
    }
    if min_base > max_base {
        return Err(ConfigError::InvalidLimit {
            detail: "MIN_CHARGE must be <= MAX_CHARGE".to_string(),
        });
    }
    Ok(ChargeLimits {
        min_base_units: min_base,
        max_base_units: max_base,
    })
}

/// Parse a comma-separated allowlist into normalized symbols.
pub fn parse_allowlist(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(normalize_symbol)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Pure expiry check: an invoice is expired once `now` is past `expires_at`
/// (both Unix seconds). Kept here so time handling has one deterministic home.
pub fn is_expired(expires_at_unix: u64, now_unix: u64) -> bool {
    now_unix > expires_at_unix
}

// ---------------------------------------------------------------------------
// Env wrappers (thin; delegate to pure resolvers)
// ---------------------------------------------------------------------------

fn env_opt(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.trim().is_empty())
}

fn env_or(var: &str, default: &str) -> String {
    env_opt(var).unwrap_or_else(|| default.to_string())
}

/// Cluster from `--cluster` flag or `SOLANA_CLUSTER` (default `devnet`), gated by
/// the `ALLOW_MAINNET` interlock.
pub fn cluster_from(flag: Option<&str>) -> Result<Cluster, ConfigError> {
    let value = flag
        .map(str::to_string)
        .unwrap_or_else(|| env_or("SOLANA_CLUSTER", "devnet"));
    let allow = env_or("ALLOW_MAINNET", "false").eq_ignore_ascii_case("true");
    resolve_cluster(&value, allow)
}

/// Cluster from environment only.
pub fn cluster_from_env() -> Result<Cluster, ConfigError> {
    cluster_from(None)
}

/// Recipient from `--recipient` flag or `MERCHANT_WALLET`.
pub fn recipient_from(flag: Option<&str>) -> Result<Pubkey, ConfigError> {
    let value = match flag {
        Some(v) => v.to_string(),
        None => env_opt("MERCHANT_WALLET").ok_or(ConfigError::Missing {
            var: "MERCHANT_WALLET",
        })?,
    };
    resolve_recipient(&value)
}

/// Commitment from `--commitment` flag or `PAYMENT_COMMITMENT` (default confirmed).
pub fn commitment_from(flag: Option<&str>) -> Result<CommitmentLevel, ConfigError> {
    let value = flag
        .map(str::to_string)
        .unwrap_or_else(|| env_or("PAYMENT_COMMITMENT", "confirmed"));
    resolve_commitment(&value)
}

/// Allowlist from `TOKEN_ALLOWLIST` (default `USDC`).
pub fn allowlist_from_env() -> Vec<String> {
    parse_allowlist(&env_or("TOKEN_ALLOWLIST", "USDC"))
}

/// Mint from `--mint` override, else per-cluster table for `token`.
pub fn mint_from(
    token: &str,
    cluster: Cluster,
    mint_flag: Option<&str>,
) -> Result<Pubkey, ConfigError> {
    resolve_mint(token, cluster, mint_flag, &allowlist_from_env())
}

/// RPC endpoints from flags or `SOLANA_RPC_PRIMARY` / `SOLANA_RPC_FALLBACK`.
pub fn rpc_endpoints_from(
    primary_flag: Option<&str>,
    fallback_flag: Option<&str>,
) -> Result<Vec<String>, ConfigError> {
    let primary = primary_flag
        .map(str::to_string)
        .or_else(|| env_opt("SOLANA_RPC_PRIMARY"));
    let fallback = fallback_flag
        .map(str::to_string)
        .or_else(|| env_opt("SOLANA_RPC_FALLBACK"));
    resolve_rpc_endpoints(primary.as_deref(), fallback.as_deref())
}

/// Charge limits from `MIN_CHARGE` / `MAX_CHARGE` (defaults 0.01 / 1000).
pub fn charge_limits_from_env() -> Result<ChargeLimits, ConfigError> {
    resolve_charge_limits(&env_or("MIN_CHARGE", "0.01"), &env_or("MAX_CHARGE", "1000"))
}

/// Store label from `--label` flag or `STORE_LABEL` (default "ZeroClaw Store").
pub fn label_from(flag: Option<&str>) -> String {
    flag.map(str::to_string)
        .unwrap_or_else(|| env_or("STORE_LABEL", "ZeroClaw Store"))
}

/// A `u64` env var with a default, validated.
pub fn u64_from_env(var: &'static str, default: u64) -> Result<u64, ConfigError> {
    match env_opt(var) {
        None => Ok(default),
        Some(v) => v
            .parse()
            .map_err(|_| ConfigError::InvalidNumber { var, value: v }),
    }
}

/// A `u32` env var with a default, validated.
pub fn u32_from_env(var: &'static str, default: u32) -> Result<u32, ConfigError> {
    match env_opt(var) {
        None => Ok(default),
        Some(v) => v
            .parse()
            .map_err(|_| ConfigError::InvalidNumber { var, value: v }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_parsing_and_mainnet_interlock() {
        assert_eq!(resolve_cluster("devnet", false), Ok(Cluster::Devnet));
        assert_eq!(
            resolve_cluster("mainnet-beta", true),
            Ok(Cluster::MainnetBeta)
        );
        assert_eq!(
            resolve_cluster("mainnet-beta", false),
            Err(ConfigError::MainnetNotAllowed)
        );
        assert!(matches!(
            resolve_cluster("testnet", true),
            Err(ConfigError::UnknownCluster { .. })
        ));
    }

    #[test]
    fn commitment_rejects_processed() {
        assert_eq!(
            resolve_commitment("confirmed"),
            Ok(CommitmentLevel::Confirmed)
        );
        assert_eq!(
            resolve_commitment("finalized"),
            Ok(CommitmentLevel::Finalized)
        );
        assert_eq!(
            resolve_commitment("processed"),
            Err(ConfigError::ProcessedNotAllowed)
        );
        assert!(matches!(
            resolve_commitment("nope"),
            Err(ConfigError::UnknownCommitment { .. })
        ));
    }

    #[test]
    fn mint_resolves_from_cluster_table() {
        let allow = vec!["USDC".to_string()];
        let devnet = resolve_mint("USDC", Cluster::Devnet, None, &allow).unwrap();
        let mainnet = resolve_mint("USDC", Cluster::MainnetBeta, None, &allow).unwrap();
        assert_eq!(devnet.to_string(), USDC_MINT_DEVNET);
        assert_eq!(mainnet.to_string(), USDC_MINT_MAINNET);
    }

    #[test]
    fn mint_override_wins_but_token_must_be_allowed() {
        let allow = vec!["USDC".to_string()];
        let override_mint = "So11111111111111111111111111111111111111112";
        let m = resolve_mint("usdc", Cluster::Devnet, Some(override_mint), &allow).unwrap();
        assert_eq!(m.to_string(), override_mint);
        // Token not in allowlist is rejected even with an override.
        assert!(matches!(
            resolve_mint("SCAM", Cluster::Devnet, Some(override_mint), &allow),
            Err(ConfigError::TokenNotAllowed { .. })
        ));
    }

    #[test]
    fn rpc_urls_require_https_except_localhost() {
        assert!(resolve_rpc_endpoints(Some("https://api.devnet.solana.com"), None).is_ok());
        assert!(resolve_rpc_endpoints(Some("http://127.0.0.1:8899"), None).is_ok());
        assert!(resolve_rpc_endpoints(Some("http://localhost:8899"), None).is_ok());
        assert!(matches!(
            resolve_rpc_endpoints(Some("http://evil.example.com"), None),
            Err(ConfigError::InvalidRpcUrl { .. })
        ));
        assert!(matches!(
            resolve_rpc_endpoints(None, None),
            Err(ConfigError::Missing {
                var: "SOLANA_RPC_PRIMARY"
            })
        ));
    }

    #[test]
    fn rpc_endpoints_are_ordered_primary_then_fallback() {
        let e = resolve_rpc_endpoints(
            Some("https://primary.example.com"),
            Some("https://fallback.example.com"),
        )
        .unwrap();
        assert_eq!(
            e,
            vec![
                "https://primary.example.com",
                "https://fallback.example.com"
            ]
        );
    }

    #[test]
    fn charge_limits_validated() {
        let l = resolve_charge_limits("0.01", "1000").unwrap();
        assert_eq!(l.min_base_units, 10_000);
        assert_eq!(l.max_base_units, 1_000_000_000);
        assert!(matches!(
            resolve_charge_limits("0", "1000"),
            Err(ConfigError::InvalidLimit { .. })
        ));
        assert!(matches!(
            resolve_charge_limits("5", "1"),
            Err(ConfigError::InvalidLimit { .. })
        ));
    }

    #[test]
    fn allowlist_parsing_normalizes() {
        assert_eq!(parse_allowlist("usdc, USDT ,,"), vec!["USDC", "USDT"]);
        assert_eq!(parse_allowlist("USDC"), vec!["USDC"]);
    }

    #[test]
    fn expiry_is_a_strict_after_comparison() {
        assert!(!is_expired(1000, 999));
        assert!(!is_expired(1000, 1000)); // exactly at expiry is not yet expired
        assert!(is_expired(1000, 1001));
    }
}
