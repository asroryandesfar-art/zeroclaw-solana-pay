//! QR rendering for a Solana Pay URL.
//!
//! # Why this exists
//! A customer pays by scanning a QR of the Solana Pay URL. This module turns a
//! validated `solana:` URL into a crisp PNG the WhatsApp channel can send.
//!
//! # Design
//! We take only the module matrix from the `qrcode` crate and rasterize it with
//! `image` ourselves, so we control the quiet zone and pixel scale directly and
//! avoid coupling to any cross-crate image-integration version. Output is always
//! PNG regardless of the file extension.
//!
//! # Security
//! The input is validated to be a `solana:` URL, so this never renders an
//! arbitrary `http(s)` link that could be a phishing target if the QR leaked
//! from an earlier step. Rendering is offline and touches no funds.

use std::path::Path;

use image::{ImageBuffer, ImageFormat, Luma};
use qrcode::{Color, QrCode};

/// Pixels per QR module when `--scale` is not given.
pub const DEFAULT_PIXEL_SCALE: u32 = 8;
/// White border width, in modules, when `--quiet-zone` is not given.
pub const DEFAULT_QUIET_ZONE: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrError {
    /// The URL is not a Solana Pay (`solana:`) URL.
    InvalidScheme,
    /// The data could not be encoded (e.g. too long for any QR version).
    Encode(String),
    /// The PNG could not be written.
    Io(String),
}

impl std::fmt::Display for QrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QrError::InvalidScheme => write!(f, "url must be a Solana Pay `solana:` URL"),
            QrError::Encode(m) => write!(f, "failed to encode QR: {m}"),
            QrError::Io(m) => write!(f, "failed to write QR image: {m}"),
        }
    }
}

impl std::error::Error for QrError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderInfo {
    /// Total modules per side including the quiet zone.
    pub modules: u32,
    /// Output image side length in pixels.
    pub pixel_size: u32,
    pub size_bytes: u64,
}

/// Render `url` to a PNG at `out_path`. `pixel_scale` is pixels per QR module;
/// `quiet_zone` is the white border in modules.
pub fn render_png(
    url: &str,
    out_path: &Path,
    pixel_scale: u32,
    quiet_zone: u32,
) -> Result<RenderInfo, QrError> {
    if !url.starts_with("solana:") {
        return Err(QrError::InvalidScheme);
    }
    let scale = pixel_scale.max(1);

    let code = QrCode::new(url.as_bytes()).map_err(|e| QrError::Encode(e.to_string()))?;
    let width = code.width();
    let colors = code.to_colors();

    let total_modules = width as u32 + 2 * quiet_zone;
    let side_px = total_modules * scale;

    // Start all-white, then paint the dark modules.
    let mut img: ImageBuffer<Luma<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(side_px, side_px, Luma([255u8]));

    for y in 0..width {
        for x in 0..width {
            if colors[y * width + x] == Color::Dark {
                let x0 = (x as u32 + quiet_zone) * scale;
                let y0 = (y as u32 + quiet_zone) * scale;
                for dy in 0..scale {
                    for dx in 0..scale {
                        img.put_pixel(x0 + dx, y0 + dy, Luma([0u8]));
                    }
                }
            }
        }
    }

    img.save_with_format(out_path, ImageFormat::Png)
        .map_err(|e| QrError::Io(e.to_string()))?;

    let size_bytes = std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0);
    Ok(RenderInfo {
        modules: total_modules,
        pixel_size: side_px,
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        // Include the process id to avoid collisions across parallel test runs.
        p.push(format!("solpay-qr-{}-{name}.png", std::process::id()));
        p
    }

    #[test]
    fn renders_a_valid_png() {
        let path = temp_path("valid");
        let info = render_png(
            "solana:So11111111111111111111111111111111111111112?amount=25&spl-token=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v&reference=So11111111111111111111111111111111111111112&label=Shop",
            &path,
            DEFAULT_PIXEL_SCALE,
            DEFAULT_QUIET_ZONE,
        )
        .unwrap();

        assert!(info.size_bytes > 0);
        assert_eq!(info.pixel_size, info.modules * DEFAULT_PIXEL_SCALE);

        // Verify the file really is a PNG (magic bytes).
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_non_solana_url() {
        let path = temp_path("bad");
        assert_eq!(
            render_png("https://evil.example.com", &path, 8, 4),
            Err(QrError::InvalidScheme)
        );
        assert!(!path.exists());
    }

    #[test]
    fn pixel_scale_is_clamped_to_at_least_one() {
        let path = temp_path("scale0");
        let info = render_png("solana:abc", &path, 0, 1).unwrap();
        assert!(info.pixel_size >= info.modules); // scale forced to >= 1
        let _ = std::fs::remove_file(&path);
    }
}
