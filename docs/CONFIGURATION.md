# Configuration

Two planes, one secrets file. Nothing dangerous is ever taken from a message.

- **App plane** — `.env` (git-ignored): the payment domain + the only secret
  values. Copy from [`.env.example`](../.env.example).
- **Runtime plane** — `agent/zeroclaw.toml` (agent phase): how ZeroClaw runs
  (LLM provider, memory, gateway, WhatsApp channel), referencing secrets by
  env-var **name** only.

Every value below is validated at startup; `solpay` **refuses to run** on a bad
value rather than booting half-configured.

## Values read by `solpay`

| Var | Meaning | Validation | Default |
|---|---|---|---|
| `SOLANA_CLUSTER` | `devnet` or `mainnet-beta` | one of the two | `devnet` |
| `ALLOW_MAINNET` | mainnet safety interlock | must be `true` to use mainnet | `false` |
| `MERCHANT_WALLET` | receiving **public** key | base58, **on-curve** (not an ATA/PDA) | — (required) |
| `STORE_LABEL` | label in wallet + QR | any text | `ZeroClaw Store` |
| `TOKEN_ALLOWLIST` | accepted symbols (comma); `USDC` (SPL) and/or `SOL` (native) | non-empty | `USDC,SOL` |
| `MIN_CHARGE` | lower bound (decimal) | `> 0`, ≤ `MAX_CHARGE` | `0.01` |
| `MAX_CHARGE` | upper bound (decimal) | ≥ `MIN_CHARGE` | `1000` |
| `PAYMENT_COMMITMENT` | settlement bar | `confirmed`/`finalized`; **`processed` rejected** | `confirmed` |
| `SOLANA_RPC_PRIMARY` | primary RPC URL | `https` (or `http` for localhost) | — (required for `verify`) |
| `SOLANA_RPC_FALLBACK` | secondary RPC URL | `https`/localhost, optional | — |
| `RPC_TIMEOUT_MS` | per-call timeout | integer | `8000` |
| `RPC_MAX_RETRIES` | extra attempts per endpoint | integer | `3` |
| `RPC_BACKOFF_BASE_MS` | backoff base (jittered) | integer | `250` |

Any of these can also be passed as a CLI flag (`--recipient`, `--mint`,
`--cluster`, `--commitment`, `--rpc`, `--rpc-fallback`, `--label`). Flags win
over the environment — this is how ZeroClaw skills pass **locked args** the model
cannot override.

## The master switch: devnet vs mainnet

`SOLANA_CLUSTER` is the single switch. It selects both the RPC targets **and**
the token mint (from a compiled per-cluster table), so you can never validate a
devnet payment against a mainnet mint. Going live is a conscious two-value
change:

```env
SOLANA_CLUSTER=mainnet-beta
ALLOW_MAINNET=true
```

With `ALLOW_MAINNET` unset/false, `solpay` refuses to start on mainnet — a demo
or a fat-fingered env can never begin accepting real funds by accident.

## Token allowlist and mints

The mint is resolved from a table, never from free text (the anti-fake-USDC seam):

| Symbol | mainnet-beta | devnet |
|---|---|---|
| `USDC` | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` |
| `SOL` | native (no mint, 9 decimals) | native (no mint, 9 decimals) |

A `--mint` override is honored for SPL tokens, but the token symbol must still be
in `TOKEN_ALLOWLIST`. `SOL` is native — the Solana Pay URL omits `spl-token` and
verification checks the lamport delta credited to the merchant wallet.

## Secret handling

- `.env` is the **only** file with secrets and is git-ignored.
- `agent/zeroclaw.toml` references secrets by env-var name (`*_env`), never inline.
- **No private keys exist** — the system is non-custodial.
- An RPC URL may embed a provider API key → treat the whole URL as a secret.
- Recommended: `chmod 600 .env`.

## Agent / SOP-layer values (wired in the agent phase)

`INVOICE_TTL_SECONDS`, `POLL_INTERVAL_SECONDS`, `POLL_MAX_ATTEMPTS`,
`WHATSAPP_TOKEN`, `WHATSAPP_VERIFY_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`,
`LLM_API_KEY`. See [`.env.example`](../.env.example) for descriptions.
