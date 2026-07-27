//! Command-line interface: argument parsing and command handlers.
//!
//! The default output is machine-readable JSON; `--format human` switches to
//! friendly text. Handlers are thin: they resolve+validate config, call the
//! library, and build an output struct. All fallible steps return [`AppError`]
//! carrying the deterministic exit code.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

use crate::config;
use crate::domain::validation::{validate_amount, validate_token};
use crate::error::AppError;
use crate::money::{format_base_units, parse_amount, USDC_DECIMALS};
use crate::output::{render, CreateUrlOutput, OutputFormat, RenderQrOutput, VerifyOutput};
use crate::qr::{self, DEFAULT_PIXEL_SCALE, DEFAULT_QUIET_ZONE};
use crate::solana::pay_url::{build_transfer_request_url, TransferRequest};
use crate::solana::pubkey::parse_pubkey;
use crate::solana::reference::generate as generate_reference;
use crate::solana::rpc::{RpcClient, UreqTransport};
use crate::solana::verify::{verify_payment, Expected, Verdict};

#[derive(Parser, Debug)]
#[command(
    name = "solpay",
    version,
    about = "Deterministic Solana Pay helper: build transfer-request URLs, render QR, and verify USDC payments. Non-custodial — never signs, never holds keys."
)]
pub struct Cli {
    /// Output format. `json` (default) is machine-readable; `human` is text.
    #[arg(long, value_enum, default_value_t = FormatArg::Json, global = true)]
    pub format: FormatArg,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum FormatArg {
    Json,
    Human,
}

impl From<FormatArg> for OutputFormat {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Json => OutputFormat::Json,
            FormatArg::Human => OutputFormat::Human,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Build an invoice's reference and Solana Pay URL (stateless, offline).
    #[command(
        after_help = "EXAMPLE:\n  MERCHANT_WALLET=<wallet> SOLANA_CLUSTER=devnet \\\n    solpay create-url --amount 25 --token USDC --message \"Table 4\""
    )]
    CreateUrl(CreateUrlArgs),

    /// Render a Solana Pay URL to a PNG QR code.
    #[command(
        after_help = "EXAMPLE:\n  solpay render-qr --url 'solana:<recipient>?amount=25&spl-token=<mint>&reference=<ref>&label=Shop' \\\n    --out /tmp/qr.png"
    )]
    RenderQr(RenderQrArgs),

    /// Verify on-chain whether an invoice has been paid.
    #[command(
        after_help = "EXAMPLE:\n  SOLANA_RPC_PRIMARY=https://api.devnet.solana.com MERCHANT_WALLET=<wallet> \\\n    solpay verify --reference <ref> --amount-base-units 25000000"
    )]
    Verify(VerifyArgs),
}

#[derive(clap::Args, Debug)]
pub struct CreateUrlArgs {
    /// Human amount, e.g. "25" or "0.5".
    #[arg(long)]
    pub amount: String,
    /// Token symbol (must be in the allowlist), e.g. "USDC".
    #[arg(long)]
    pub token: String,
    /// Reuse a specific reference (base58). If omitted, a secure one is generated.
    #[arg(long)]
    pub reference: Option<String>,
    /// Label shown in wallets. Defaults to STORE_LABEL.
    #[arg(long)]
    pub label: Option<String>,
    /// Free-text message/memo, e.g. "Table 4".
    #[arg(long)]
    pub message: Option<String>,
    /// Merchant wallet (locked). Defaults to MERCHANT_WALLET.
    #[arg(long)]
    pub recipient: Option<String>,
    /// Token mint override (locked). Defaults to the per-cluster mint.
    #[arg(long)]
    pub mint: Option<String>,
    /// Cluster (locked). Defaults to SOLANA_CLUSTER.
    #[arg(long)]
    pub cluster: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct RenderQrArgs {
    /// The Solana Pay URL to encode (must start with `solana:`).
    #[arg(long)]
    pub url: String,
    /// Output PNG path.
    #[arg(long)]
    pub out: PathBuf,
    /// Pixels per QR module.
    #[arg(long)]
    pub scale: Option<u32>,
    /// White border in modules.
    #[arg(long)]
    pub quiet_zone: Option<u32>,
}

#[derive(clap::Args, Debug)]
pub struct VerifyArgs {
    /// The invoice's reference (base58).
    #[arg(long)]
    pub reference: String,
    /// Exact expected amount in base units.
    #[arg(long)]
    pub amount_base_units: u64,
    /// Merchant wallet (locked). Defaults to MERCHANT_WALLET.
    #[arg(long)]
    pub recipient: Option<String>,
    /// Token mint override (locked). Defaults to the per-cluster USDC mint.
    #[arg(long)]
    pub mint: Option<String>,
    /// Cluster (locked). Defaults to SOLANA_CLUSTER.
    #[arg(long)]
    pub cluster: Option<String>,
    /// Commitment threshold: `confirmed` or `finalized`. Defaults to PAYMENT_COMMITMENT.
    #[arg(long)]
    pub commitment: Option<String>,
    /// Primary RPC URL (locked). Defaults to SOLANA_RPC_PRIMARY.
    #[arg(long)]
    pub rpc: Option<String>,
    /// Fallback RPC URL (locked). Defaults to SOLANA_RPC_FALLBACK.
    #[arg(long)]
    pub rpc_fallback: Option<String>,
    /// Max signatures to inspect for the reference.
    #[arg(long)]
    pub signature_limit: Option<u32>,
}

/// Parse args and run, returning the rendered output string on success.
pub fn dispatch(command: Command, format: OutputFormat) -> Result<String, AppError> {
    match command {
        Command::CreateUrl(a) => Ok(render(&run_create_url(a)?, format)),
        Command::RenderQr(a) => Ok(render(&run_render_qr(a)?, format)),
        Command::Verify(a) => Ok(render(&run_verify(a)?, format)),
    }
}

fn run_create_url(a: CreateUrlArgs) -> Result<CreateUrlOutput, AppError> {
    let cluster = config::cluster_from(a.cluster.as_deref())?;
    let recipient = config::recipient_from(a.recipient.as_deref())?;

    // The token symbol comes from the user/message, so an unsupported token is
    // *invalid input* (exit 2) — validate it here, before the config-level mint
    // resolution, so it is never misclassified as a config error (exit 3).
    validate_token(&a.token, &config::allowlist_from_env())?;
    let mint = config::mint_from(&a.token, cluster, a.mint.as_deref())?;

    let amount_base = parse_amount(&a.amount, USDC_DECIMALS)?;
    let limits = config::charge_limits_from_env()?;
    validate_amount(amount_base, &limits)?;
    let amount_ui = format_base_units(amount_base, USDC_DECIMALS);

    // Reuse a provided reference (deterministic) or generate a secure one.
    let reference = match a.reference.as_deref() {
        Some(r) => parse_pubkey(r)?,
        None => generate_reference().map_err(|e| AppError::internal(e.to_string()))?,
    };

    let label = config::label_from(a.label.as_deref());
    let url = build_transfer_request_url(&TransferRequest {
        recipient: &recipient,
        amount_ui: &amount_ui,
        spl_token: &mint,
        reference: &reference,
        label: &label,
        message: a.message.as_deref(),
    });

    Ok(CreateUrlOutput {
        reference: reference.to_string(),
        url,
        recipient: recipient.to_string(),
        mint: mint.to_string(),
        token: crate::domain::validation::normalize_symbol(&a.token),
        cluster: cluster.as_str().to_string(),
        amount_base_units: amount_base,
        amount_ui,
        label,
        message: a.message,
    })
}

fn run_render_qr(a: RenderQrArgs) -> Result<RenderQrOutput, AppError> {
    let scale = a.scale.unwrap_or(DEFAULT_PIXEL_SCALE);
    let quiet = a.quiet_zone.unwrap_or(DEFAULT_QUIET_ZONE);
    let info = qr::render_png(&a.url, &a.out, scale, quiet).map_err(map_qr_error)?;
    Ok(RenderQrOutput {
        image_path: a.out.display().to_string(),
        format: "png".to_string(),
        size_bytes: info.size_bytes,
        modules: info.modules,
        pixel_size: info.pixel_size,
    })
}

fn map_qr_error(e: qr::QrError) -> AppError {
    match e {
        // Input problems (bad scheme / data too long) are invalid input.
        qr::QrError::InvalidScheme | qr::QrError::Encode(_) => {
            AppError::invalid_input(e.to_string())
        }
        qr::QrError::Io(_) => AppError::internal(e.to_string()),
    }
}

fn run_verify(a: VerifyArgs) -> Result<VerifyOutput, AppError> {
    let cluster = config::cluster_from(a.cluster.as_deref())?;
    let recipient = config::recipient_from(a.recipient.as_deref())?;
    // v1 verifies USDC; an explicit --mint override is still honored.
    let mint = config::mint_from("USDC", cluster, a.mint.as_deref())?;
    let commitment = config::commitment_from(a.commitment.as_deref())?;
    let endpoints = config::rpc_endpoints_from(a.rpc.as_deref(), a.rpc_fallback.as_deref())?;
    let reference = parse_pubkey(&a.reference)?;

    let timeout = Duration::from_millis(config::u64_from_env("RPC_TIMEOUT_MS", 8_000)?);
    let retries = config::u32_from_env("RPC_MAX_RETRIES", 3)?;
    let backoff = Duration::from_millis(config::u64_from_env("RPC_BACKOFF_BASE_MS", 250)?);
    let limit = a.signature_limit.unwrap_or(20);

    let expected = Expected::new(reference, mint, recipient, a.amount_base_units, commitment);
    let client = RpcClient::new(UreqTransport::new(timeout), endpoints, retries, backoff);

    let verdict = verify_payment(&client, &expected, limit)?;
    Ok(match verdict {
        Verdict::Paid { signature, slot } => VerifyOutput::paid(signature, slot),
        Verdict::Mismatch { reason } => VerifyOutput::mismatch(reason),
        Verdict::Pending => VerifyOutput::pending(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_and_verifies_argument_structure() {
        // clap's own derive invariants (no conflicting flags, valid structure).
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
