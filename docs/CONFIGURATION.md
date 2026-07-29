# Configuration

Three places, clean separation. Nothing dangerous is ever taken from a message.
The ZeroClaw layer is validated against **ZeroClaw v0.8.3** (schema_version 3).

- **Money-path config** — `~/.zeroclaw/solpay.env` (never committed): the locked
  values `solpay` reads (wallet, cluster, mint table, RPC, limits). Copy from
  [`agent/solpay.env.example`](../agent/solpay.env.example). The skills **source
  this file** at run time — see [Why a file, not env vars](#why-a-file-not-env-vars).
- **Runtime config** — `agent/config.toml` (deployed by `scripts/setup.sh`): how
  ZeroClaw runs (LLM provider, memory ledger, gateway, WhatsApp channel, risk
  profile, skills, SOPs). Contains **no secrets**.
- **Secrets** — set with `zeroclaw config set <path>` (masked input, **encrypted
  at rest** via `[secrets] encrypt = true` + the config dir's `.secret_key`):
  the WhatsApp tokens and the LLM API key. ZeroClaw has no `*_env` reference
  scheme; this encrypted store is its native secret model.

Every `solpay` value below is validated at startup; `solpay` **refuses to run**
on a bad value rather than booting half-configured.

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
`--cluster`, `--commitment`, `--rpc`, `--rpc-fallback`, `--label`). The skills
leave these flags **out** of the command template and let `solpay` read them from
`solpay.env`, so the model can supply only `amount`, `token`, and `message` — it
can never reach the wallet, mint, or cluster.

## Why a file, not env vars

ZeroClaw runs a skill's shell command with a **cleared environment** — only
`PATH`, `HOME`, `TERM`, and a few locale vars survive (`SAFE_ENV_VARS` in
ZeroClaw's `skill_tool.rs`). Ambient env vars like `MERCHANT_WALLET` would be
stripped, so each money-path skill sources `~/.zeroclaw/solpay.env` (reachable
via the surviving `$HOME`) before calling `solpay`:

```toml
command = "set -a && . \"${HOME}/.zeroclaw/solpay.env\" && solpay create-url --amount {{amount}} --token {{token}} --message {{message}}"
```

Because the file is sourced by `/bin/sh`, any value containing spaces must be
quoted (see `solpay.env.example`).

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

- **No private keys exist** — the system is non-custodial; `solpay.env` holds only
  a **public** receiving key and public parameters, so it is not itself a secret
  (though `chmod 600` is still recommended).
- The real secrets — WhatsApp `access_token`/`app_secret`/`verify_token` and the
  LLM `api_key` — are set with `zeroclaw config set` and **encrypted at rest** in
  the config dir (`[secrets] encrypt = true` + `.secret_key`). They never appear
  in any committed file.
- An RPC URL may embed a provider API key → if so, treat that URL as a secret and
  keep it in `solpay.env` (mode 600), not in git.

## Agent / SOP-layer values

- WhatsApp + LLM secrets: set via `zeroclaw config set` (see above).
- Staff allowlist: `peer_groups.whatsapp_staff.external_peers` in `config.toml`.
- Invoice TTL / polling cadence: expressed in the SOPs (`agent/sops/*/SOP.md`)
  and the `verify-payments` cron trigger.
