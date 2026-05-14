# Changelog

All notable changes to soter will be documented in this file.

## [Unreleased]

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
- Defaulted the CVC5 source build to `--no-poly` for a portable first FFI path.
