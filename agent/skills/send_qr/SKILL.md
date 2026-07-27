---
name: send_qr
description: Render a Solana Pay URL to a PNG QR code.
version: 1.0.0
---

# send_qr

Render the invoice's Solana Pay URL to an image the WhatsApp channel can send.

Run:

```
solpay render-qr --url <URL> --out agent/data/tmp/<REFERENCE>.png
```

- `URL` is the `url` field from `create_invoice`.
- Write to a per-reference path under `agent/data/tmp/` so files don't collide.

On success (exit 0) it prints JSON with `image_path`. Send that image to the
customer with a short caption (invoice number, amount, "scan to pay, expires in
N minutes"). Exit `2` means the URL was not a valid `solana:` URL.
