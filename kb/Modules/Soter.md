---
tags: [module, smt, cvc5]
source: mixed
date: 2026-05-14
---
# Soter

`soter` owns SMT-backed safety and policy assurance as a reusable Converge
extension.

## Owns

- SMT query and report types.
- Fake backend for CI and deterministic tests.
- Native CVC5 FFI boundary.
- SMT suggestor surface.
- Typed abstract Arbiter invariant fixtures pulled by high-risk policy claims.
- Formation capability descriptors.
- Searched-evidence vocabulary.
- Shared Converge `execution_identity` on reports for native version/build/
  config audit.

## Public Surface

- `SmtQuery`
- `SmtReport`
- `SmtStatus`
- `SmtBackend`
- `FakeSmtBackend`
- `ArbiterExpenseCommitInvariant`
- `ArbiterExpensePolicyModel`
- `SmtSuggestor`
- `ProvenanceSource`
- `formation_capabilities()`
- `Cvc5FfiBackend` behind the `cvc5` feature

## Entry Points

- `soter-smt/README.md`
- `soter-smt/crates/soter/src/lib.rs`
- `soter-smt/crates/soter/src/types.rs`
- `soter-smt/crates/soter/src/suggestor.rs`
- `soter-smt/crates/cvc5-sys/build.rs`
- `soter-smt/Makefile`

## Boundary

Converge owns promotion. Soter emits evidence. Arbiter remains the runtime
policy gate for Cedar decisions. Soter's Arbiter fixtures are generated
abstractions for counterexample search; they are not a replacement for Cedar
runtime evaluation or full Cedar semantics.
