//! Solana Pay transfer-request URL builder.
//!
//! Format (per the Solana Pay spec):
//! ```text
//! solana:<recipient>?amount=<ui>&spl-token=<mint>&reference=<ref>&label=<label>&message=<msg>
//! ```
//! * `recipient` is the merchant's on-curve wallet (the wallet derives the ATA
//!   from it and `spl-token`).
//! * `amount` is a uiAmountString (human units, e.g. "25" or "0.5").
//! * base58 fields are URL-safe; only `label`/`message` (free text) are
//!   percent-encoded.

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use solana_pubkey::Pubkey;

pub struct TransferRequest<'a> {
    pub recipient: &'a Pubkey,
    pub amount_ui: &'a str,
    /// `Some(mint)` for an SPL-token (e.g. USDC) transfer; `None` for native SOL.
    pub spl_token: Option<&'a Pubkey>,
    pub reference: &'a Pubkey,
    pub label: &'a str,
    pub message: Option<&'a str>,
}

/// Build the Solana Pay URL. With `spl_token = Some(mint)` this is an SPL-token
/// (USDC) request; with `None` it is a native SOL request (the `spl-token`
/// parameter is omitted, per the Solana Pay spec).
pub fn build_transfer_request_url(req: &TransferRequest) -> String {
    let label_enc = utf8_percent_encode(req.label, NON_ALPHANUMERIC);
    let mut url = format!(
        "solana:{recipient}?amount={amount}",
        recipient = req.recipient,
        amount = req.amount_ui,
    );
    if let Some(mint) = req.spl_token {
        url.push_str(&format!("&spl-token={mint}"));
    }
    url.push_str(&format!(
        "&reference={reference}",
        reference = req.reference
    ));
    url.push_str(&format!("&label={label_enc}"));
    if let Some(msg) = req.message {
        let msg_enc = utf8_percent_encode(msg, NON_ALPHANUMERIC);
        url.push_str("&message=");
        url.push_str(&msg_enc.to_string());
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const RECIPIENT: &str = "So11111111111111111111111111111111111111112";

    fn pk(s: &str) -> Pubkey {
        Pubkey::from_str(s).unwrap()
    }

    fn req<'a>(
        label: &'a str,
        message: Option<&'a str>,
        recip: &'a Pubkey,
        mint: &'a Pubkey,
        reference: &'a Pubkey,
    ) -> TransferRequest<'a> {
        TransferRequest {
            recipient: recip,
            amount_ui: "25",
            spl_token: Some(mint),
            reference,
            label,
            message,
        }
    }

    #[test]
    fn builds_spec_compliant_url() {
        let (r, m, rf) = (pk(RECIPIENT), pk(USDC_MINT), pk(RECIPIENT));
        let url = build_transfer_request_url(&req("Store", None, &r, &m, &rf));
        assert!(url.starts_with("solana:So11111111111111111111111111111111111111112?"));
        assert!(url.contains("amount=25"));
        assert!(url.contains(&format!("spl-token={USDC_MINT}")));
        assert!(url.contains("reference="));
        assert!(url.contains("label=Store"));
        assert!(!url.contains("message="));
    }

    #[test]
    fn percent_encodes_label_and_message() {
        let (r, m, rf) = (pk(RECIPIENT), pk(USDC_MINT), pk(RECIPIENT));
        let url =
            build_transfer_request_url(&req("Blue Bottle Coffee", Some("Table 4"), &r, &m, &rf));
        // Spaces must be encoded, not raw.
        assert!(url.contains("label=Blue%20Bottle%20Coffee"));
        assert!(url.contains("message=Table%204"));
        assert!(!url.contains("Blue Bottle"));
    }

    #[test]
    fn amount_is_passed_through_as_ui_string() {
        let (r, m, rf) = (pk(RECIPIENT), pk(USDC_MINT), pk(RECIPIENT));
        let mut request = req("S", None, &r, &m, &rf);
        request.amount_ui = "0.5";
        let url = build_transfer_request_url(&request);
        assert!(url.contains("amount=0.5"));
    }

    #[test]
    fn native_sol_url_omits_spl_token() {
        let (r, rf) = (pk(RECIPIENT), pk(RECIPIENT));
        let url = build_transfer_request_url(&TransferRequest {
            recipient: &r,
            amount_ui: "1",
            spl_token: None, // native SOL
            reference: &rf,
            label: "Store",
            message: None,
        });
        assert!(url.starts_with("solana:So11111111111111111111111111111111111111112?"));
        assert!(url.contains("amount=1"));
        assert!(
            !url.contains("spl-token="),
            "native SOL must not carry spl-token"
        );
        assert!(url.contains("reference="));
        assert!(url.contains("label=Store"));
    }
}
