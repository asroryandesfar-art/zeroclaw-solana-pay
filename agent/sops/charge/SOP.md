# Charge

Turns a WhatsApp message like `Charge Table 4 25 USDC` into a Solana Pay invoice + QR.
Deterministic: the LLM only extracts intent; recipient/mint/cluster are locked in the
environment and never model-controlled.

## Steps

1. **Extract intent** — Parse the inbound message into amount, token, and message. Reject anything that is not a charge intent. Never produce wallet, mint, or cluster fields.
2. **Create invoice** — Call create_invoice with the extracted amount, token, and message. It returns a reference and a solana: URL.
   - tools: create-invoice__create_invoice
3. **Render QR** — Render the URL from step 2 to a PNG.
   - tools: send-qr__render_qr
4. **Reply to staff** — Send the QR image plus the invoice reference back, and persist the invoice as PENDING in memory.
