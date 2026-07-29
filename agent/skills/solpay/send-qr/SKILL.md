---
name: send-qr
description: Render a Solana Pay URL to a PNG QR code for sending over WhatsApp.
version: 1.0.0
---

# Send QR

The `render_qr` tool rasterizes a `solana:` payment URL to a PNG. It runs fully
offline and `solpay` rejects any URL that is not a `solana:` URL, so it can never
encode an arbitrary link.
