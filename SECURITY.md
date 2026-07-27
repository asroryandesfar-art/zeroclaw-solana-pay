# Security policy

## Reporting a vulnerability

Please **do not** open a public issue for security reports. Instead, report
privately via GitHub Security Advisories ("Report a vulnerability") on this
repository, or contact the maintainers directly.

Include: affected version/commit, a description, reproduction steps, and impact.
We aim to acknowledge within a few days and will coordinate a fix and disclosure.

## Scope and design notes

This system is **non-custodial**: it holds no private key and cannot move funds.
The worst case from a host compromise is misleading invoice display or denial of
service, not theft. See [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) for trust
boundaries, threats, mitigations, and residual risks.

Secrets live only in a git-ignored `.env`; configuration references them by
env-var name. Never commit `.env`, `agent/data/`, or `agent/logs/`.
