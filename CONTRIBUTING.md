# Contributing

Thanks for your interest. This project holds money-critical code, so the bar is
high and the rules are simple.

## Ground rules

- **No floating-point arithmetic for money.** Amounts are integer base units.
- **No panics in library/production code.** Validate external input; return typed
  errors with the correct exit code.
- **Determinism.** Business rules must be pure and testable; keep network in
  `solana/rpc.rs` and decisions in `solana/verify.rs`.
- **Every public function has tests.** Prefer hermetic (offline) tests; gate
  network tests behind `#[ignore]`.
- **Security decisions are documented** in code comments and, when structural, in
  an ADR under `docs/adr/`.

## Before you open a PR

```bash
make check      # fmt-check + clippy -D warnings + tests
```

All three must pass; CI runs the same. Keep diffs focused; match the style and
comment density of the surrounding code.

## Commits & PRs

- Small, reviewable commits with clear messages.
- Describe the change, the rationale, and any threat-model impact.
- New structural decisions → add a numbered ADR.

## License

By contributing, you agree your contributions are dual-licensed under MIT OR
Apache-2.0, the same as the project.
