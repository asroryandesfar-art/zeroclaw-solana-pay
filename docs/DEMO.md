# 3-minute demo script

Goal: prove this is a **real, running** use case — a WhatsApp message becomes a
real on-chain USDC payment — while landing the non-custodial pitch. Hard cut at
3:00. Narration in English (captions on); record on **devnet**.

## ⚠️ Devnet vs Mainnet — the wallet's network (critical)

A Solana Pay transfer-request URL has **no cluster field** (see the
[spec](https://docs.solanapay.com/spec)): the parameters are only
`amount`, `spl-token`, `reference`, `label`, `message`, `memo`. **The QR cannot
force devnet — the wallet chooses the network, and Phantom defaults to
mainnet-beta.** So if Phantom shows a *mainnet* payment, it is because Phantom is
set to mainnet, not because of anything in the URL.

Before scanning, the payer must switch Phantom to devnet:

> Phantom → **Settings → Developer Settings → Testnet Mode = ON**, then select
> **Solana Devnet**. The network badge should read **Devnet**.

Safety: this demo's token mint (`4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`)
is a token **only on devnet** — on mainnet that address is not a token at all —
and the mainnet USDC mint is never placed in a devnet URL (it requires the
`ALLOW_MAINNET=true` interlock). A real mainnet USDC payment from this QR is
therefore impossible.

## Pre-flight checklist (do NOT skip)

```
[ ] Rehearse the full flow 2–3× right before recording (same devnet setup)
[ ] Payer wallet funded: devnet SOL (faucet) + devnet USDC; ATA already exists
[ ] Reliable RPC set (SOLANA_RPC_PRIMARY = a provider, not the public endpoint)
[ ] POLL_INTERVAL_SECONDS=3, PAYMENT_COMMITMENT=confirmed (fast on camera)
[ ] Agent running, scripts/verify_env.sh green, WhatsApp paired
[ ] Terminal clean, large font, secrets redacted (RPC URL, wallet)
[ ] Record MULTIPLE takes; have a backup take ready
```

## Shot layout

Three panes: **phone** (WhatsApp, via scrcpy or WhatsApp Web), **terminal**
(agent logs), **browser** (Solana Explorer, devnet). Overlay the architecture
diagram at 0:20 and text callouts at the key moments.

## Minute-by-minute

| Time | On screen | Narration |
|---|---|---|
| 0:00–0:20 | flow animation | "A coffee shop gets a WhatsApp: *Charge Table 4, 25 USDC*. Seconds later — a QR, a real on-chain payment, and a receipt. Run by an AI agent that **never touches your money**." |
| 0:20–0:40 | two-plane diagram | "Built on ZeroClaw and Solana Pay. The LLM only reads language — every payment decision is deterministic code. No private keys, ever." |
| 0:40–1:40 | phone + terminal + explorer | "Live: I message the agent…" → QR appears → "I scan and pay 25 USDC on devnet…" → explorer shows `confirmed` → "…the agent detects it and confirms." Callout: **Invoice #124 Paid ✅** |
| 1:40–2:20 | terminal + callouts | "Why safe? The agent holds only a *receiving public key*. A fake 'USDC'? Rejected — it checks the exact mint. Underpayment? Rejected. Replays? Impossible — one reference, one invoice." |
| 2:20–2:50 | `make check` + CI badge | "Clone, set six variables, run. 107 tests, deterministic and offline. CI green. Every decision documented in ADRs." |
| 2:50–3:00 | repo URL | "An AI agent that can take money — but can never touch it. Repo and docs below." |

## A CLI-only fallback demo (if WhatsApp/agent is flaky on the day)

`scripts/demo.sh` shows the same engine without ZeroClaw, and is 100% reliable:

```
scripts/demo.sh 25      # create-url → render-qr → verify (PENDING)
# pay /tmp/solpay-demo.png from a devnet USDC wallet, then:
solpay verify --reference <ref> --amount-base-units 25000000 \
  --rpc $SOLANA_RPC_PRIMARY      # → PAID ✅
```

Show the QR, the payment in a wallet, the explorer confirmation, and the verdict
flipping `pending → paid`. This proves the money path end-to-end on real devnet.

## Deliverables

- MP4 ≤ 3:00, 1080p, clear audio, English captions.
- Thumbnail with the non-custodial tagline.
- Uploaded (YouTube/Loom) and linked from the README and the submission form.
