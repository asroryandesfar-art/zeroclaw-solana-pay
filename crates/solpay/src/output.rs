//! Stable, documented output schemas for every command.
//!
//! The default is machine-readable JSON (compact, one object on stdout) so a
//! ZeroClaw skill can capture and parse it. A human mode renders the same data
//! as friendly text. Field names are a stable contract: fields are only ever
//! added, never renamed or removed, and optional values are `null` rather than
//! absent so the shape is constant.
//!
//! ## `create-url`
//! ```json
//! {
//!   "reference": "<base58>",
//!   "url": "solana:<recipient>?amount=..&spl-token=..&reference=..&label=..",
//!   "recipient": "<base58 merchant wallet>",
//!   "mint": "<base58 token mint>",
//!   "token": "USDC",
//!   "cluster": "devnet",
//!   "amount_base_units": 25000000,
//!   "amount_ui": "25",
//!   "label": "My Store",
//!   "message": "Table 4"   // or null
//! }
//! ```
//!
//! ## `render-qr`
//! ```json
//! { "image_path": "/path/qr.png", "format": "png",
//!   "size_bytes": 1234, "modules": 33, "pixel_size": 264 }
//! ```
//!
//! ## `verify`
//! ```json
//! { "status": "paid" | "pending" | "mismatch",
//!   "signature": "<base58>" | null,
//!   "slot": 123456 | null,
//!   "reason": "underpaid: received .." | null }
//! ```

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Human,
}

/// Render any output value in the chosen format.
pub fn render<T: Serialize + Human>(value: &T, format: OutputFormat) -> String {
    match format {
        // `to_string` cannot fail for these plain structs, but we never unwrap:
        // a serialization error degrades to a clearly-invalid marker instead of
        // panicking.
        OutputFormat::Json => {
            serde_json::to_string(value).unwrap_or_else(|_| "{\"error\":\"serialize\"}".to_string())
        }
        OutputFormat::Human => value.to_human(),
    }
}

/// Human-readable rendering, complementary to the JSON `Serialize`.
pub trait Human {
    fn to_human(&self) -> String;
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateUrlOutput {
    pub reference: String,
    pub url: String,
    pub recipient: String,
    pub mint: String,
    pub token: String,
    pub cluster: String,
    pub amount_base_units: u64,
    pub amount_ui: String,
    pub label: String,
    pub message: Option<String>,
}

impl Human for CreateUrlOutput {
    fn to_human(&self) -> String {
        let mut s = format!(
            "Invoice for {} {} ({} base units)\n  cluster:   {}\n  recipient: {}\n  mint:      {}\n  reference: {}\n  url:       {}",
            self.amount_ui,
            self.token,
            self.amount_base_units,
            self.cluster,
            self.recipient,
            self.mint,
            self.reference,
            self.url,
        );
        if let Some(m) = &self.message {
            s.push_str(&format!("\n  message:   {m}"));
        }
        s
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderQrOutput {
    pub image_path: String,
    pub format: String,
    pub size_bytes: u64,
    pub modules: u32,
    pub pixel_size: u32,
}

impl Human for RenderQrOutput {
    fn to_human(&self) -> String {
        format!(
            "QR written to {} ({} bytes, {}x{} px)",
            self.image_path, self.size_bytes, self.pixel_size, self.pixel_size
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyOutput {
    pub status: String,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub reason: Option<String>,
}

impl VerifyOutput {
    pub fn paid(signature: String, slot: u64) -> Self {
        Self {
            status: "paid".into(),
            signature: Some(signature),
            slot: Some(slot),
            reason: None,
        }
    }
    pub fn pending() -> Self {
        Self {
            status: "pending".into(),
            signature: None,
            slot: None,
            reason: None,
        }
    }
    pub fn mismatch(reason: String) -> Self {
        Self {
            status: "mismatch".into(),
            signature: None,
            slot: None,
            reason: Some(reason),
        }
    }
}

impl Human for VerifyOutput {
    fn to_human(&self) -> String {
        match self.status.as_str() {
            "paid" => format!(
                "PAID  signature={} slot={}",
                self.signature.as_deref().unwrap_or("?"),
                self.slot
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "?".into()),
            ),
            "mismatch" => {
                format!(
                    "MISMATCH  {}",
                    self.reason.as_deref().unwrap_or("unknown reason")
                )
            }
            _ => "PENDING".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_output_json_shape_is_stable() {
        let paid = VerifyOutput::paid("sig123".into(), 42);
        let json = render(&paid, OutputFormat::Json);
        assert_eq!(
            json,
            r#"{"status":"paid","signature":"sig123","slot":42,"reason":null}"#
        );

        let pending = VerifyOutput::pending();
        let json = render(&pending, OutputFormat::Json);
        assert_eq!(
            json,
            r#"{"status":"pending","signature":null,"slot":null,"reason":null}"#
        );

        let mismatch = VerifyOutput::mismatch("underpaid".into());
        let json = render(&mismatch, OutputFormat::Json);
        assert_eq!(
            json,
            r#"{"status":"mismatch","signature":null,"slot":null,"reason":"underpaid"}"#
        );
    }

    #[test]
    fn human_mode_is_readable() {
        assert!(
            render(&VerifyOutput::paid("s".into(), 1), OutputFormat::Human).starts_with("PAID")
        );
        assert_eq!(
            render(&VerifyOutput::pending(), OutputFormat::Human),
            "PENDING"
        );
    }

    #[test]
    fn create_url_json_includes_all_fields() {
        let out = CreateUrlOutput {
            reference: "r".into(),
            url: "solana:x".into(),
            recipient: "rec".into(),
            mint: "m".into(),
            token: "USDC".into(),
            cluster: "devnet".into(),
            amount_base_units: 25_000_000,
            amount_ui: "25".into(),
            label: "Shop".into(),
            message: None,
        };
        let json = render(&out, OutputFormat::Json);
        for key in [
            "reference",
            "url",
            "recipient",
            "mint",
            "token",
            "cluster",
            "amount_base_units",
            "amount_ui",
            "label",
            "message",
        ] {
            assert!(json.contains(&format!("\"{key}\"")), "missing key {key}");
        }
        assert!(json.contains("\"message\":null"));
    }
}
