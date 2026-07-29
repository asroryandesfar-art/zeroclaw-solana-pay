# ADR 0005 — Validate the agent layer against a real ZeroClaw runtime

**Status:** Accepted

## Context

The `solpay` engine was proven against real devnet from day one, but the ZeroClaw
agent layer (config, skills, SOPs) was originally authored from documentation and
carried `[verify]` caveats: its exact schema had not been checked against a real
ZeroClaw build. Inferred formats are a liability for a payments assistant.

## Decision

Install ZeroClaw (v0.8.3, `schema_version 3`) and validate the entire agent layer
with the real binary — `zeroclaw config list/doctor`, `zeroclaw skills audit/list`,
`zeroclaw sop validate` — iterating until green. Ship only formats the runtime
accepts.

## What validation found (and we fixed)

- **Config schema.** The real config is `config.toml` (`schema_version = 3`) with
  `[agents.*]`, `[providers.models.*]`, `[channels.*]`, `[risk_profiles.*]`,
  `[skill_bundles.*]`, `[peer_groups.*]`. The old top-level `default_provider` /
  `[channels_config.whatsapp]` / `[autonomy]` keys do not exist. Rewritten.
- **Secrets.** ZeroClaw has **no `*_env` reference scheme**; secrets are set with
  `zeroclaw config set` and encrypted at rest (`[secrets] encrypt = true`).
  Adopted its native model.
- **Skills.** The real manifest is `[skill]` + top-level `[[tools]]` with
  `kind`/`command`/`args`/`timeout_secs`. `locked_args` applies only to
  `builtin`/`mcp` tools. The old `[[skill.tools]]` / `timeout` / array-style
  `locked_args` were wrong. Rewritten; `zeroclaw skills audit` passes.
- **Environment is cleared.** The decisive finding: ZeroClaw runs a skill's shell
  command with `env_clear()`, keeping only `SAFE_ENV_VARS` (`PATH`, `HOME`, …).
  Our "solpay reads its config from the ambient environment" design would have
  failed at runtime (exit 3, config missing). Fixed by sourcing
  `~/.zeroclaw/solpay.env` (reachable via the surviving `$HOME`) inside the
  command. Proven end-to-end under a simulated `env_clear`: the skill command
  emits a valid devnet Solana Pay URL.
- **SOPs.** The real format is a per-SOP directory with `SOP.toml` (`[sop]` meta +
  `[[triggers]]`, `type`-tagged) and `SOP.md` (steps under a `## Steps` heading).
  The old single `*.yaml` files were wrong. Rewritten; `zeroclaw sop validate`
  passes for both `charge` and `verify-payments`.

## Consequences

- **+** The agent layer is no longer inferred — it loads and validates on a real
  ZeroClaw runtime; the `[verify]` caveats are removed.
- **+** `scripts/setup.sh` deploys the layer into a config dir and self-validates.
- **−** Pinned to the v0.8.3 schema; a future major ZeroClaw release may require
  `zeroclaw config migrate` and a re-validation pass.
