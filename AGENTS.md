# Soter Agent Guide

This is the canonical agent entrypoint for `soter`.

`soter` is a Converge extension for SMT-backed safety and policy assurance.
It owns SMT query types, native CVC5 bindings, solver reports, and
solver-backed suggestors.

## Start Here

1. Read `README.md`.
2. Read `kb/Home.md`.
3. Check `Cargo.toml` and feature flags before enabling native CVC5.
4. Use `just --list` for local commands.

## Commands

```bash
just check       # pure Rust/default checks
just test        # pure Rust/default tests
just lint        # formatting plus clippy
just deps        # build pinned native CVC5
just check-cvc5  # checks with the native CVC5 feature
```

## Boundaries

- Converge owns the suggestor contract and promotion path.
- `soter` owns SMT query modeling, CVC5 FFI, SMT reports, and SMT suggestors.
- Arbiter owns Cedar policy runtime behavior. Soter can analyze encoded policy
  invariants, but it does not decide runtime authorization.
- Products decide which invariants require SMT evidence and whether CVC5 runs
  in PR, nightly, or release gates.

## Rules

- Keep unsafe native FFI isolated in `crates/cvc5-sys`.
- Do not label SMT results as formal proof. They are `Searched` evidence
  unless an independent proof checker verifies an emitted artifact.
- Default checks must not require native CVC5.
- Native CVC5 builds must be explicit through `just deps` and `soter/cvc5`.
- Keep SMT-LIB rendering deterministic and hashable.
- Update `README.md` and `kb/` when supported theories, evidence semantics,
  or Formation-facing capabilities change.
