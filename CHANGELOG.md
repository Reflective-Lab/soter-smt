# Changelog

All notable changes to soter will be documented in this file.

## [Unreleased]

## [0.2.0] - 2026-05-15

### Added

- Added initial `soter-smt` extension scaffold.
- Added `converge-soter-smt` with SMT query/report types, fake backend, typed
  provenance, and an SMT suggestor.
- Added `converge-soter-cvc5-sys` as the isolated native CVC5 FFI boundary.
- Added pinned CVC5 source-build recipe under `make cvc5`.
- Added a native CVC5 version-smoke test behind the `cvc5` feature.
- Added CVC5-backed SMT-LIB `check-sat` execution with SAT, UNSAT, UNKNOWN,
  TIMEOUT, and ERROR status mapping.
- Added optional CVC5 model and unsat-core extraction.
- Added native SAT, UNSAT, and invalid-SMT-LIB tests behind the `cvc5` feature.
- Added `ArbiterExpenseCommitInvariant` and `ArbiterExpensePolicyModel` for the
  first typed abstract Arbiter counterexample fixture.
- Added property tests for deterministic Arbiter invariant rendering and native
  CVC5 tests for strict/no-violation and broken/counterexample cases.
- Defaulted the CVC5 source build to `--no-poly` for a portable first FFI path.
- `SmtReport` now carries shared Converge `ExecutionIdentity` metadata so fake
  and native CVC5 solver runs are auditable through the same contract.
- Added a local `just security-audit` gate for release hygiene.
