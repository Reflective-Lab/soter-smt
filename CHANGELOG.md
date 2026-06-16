# Changelog

All notable changes to soter will be documented in this file.

## [Unreleased]

## [0.3.0] - 2026-06-16

### Added

- `RemoteSmtBackend` (gRPC client) behind the new `remote` Cargo feature on
  `converge-soter-smt`. Implements `SmtBackend` by calling a deployed
  `soter-server` over tonic. Injects `x-converge-app` + `x-request-id`
  metadata; maps `SmtStatusCode` ↔ `SmtStatus`. Per spec §5.3.
- `crates/soter-server/` — new gRPC service hosting `Cvc5FfiBackend`.
  Tonic 0.14 + tonic-health + tonic-reflection + structured JSON logs +
  per-RPC tracing spans. Mirrors the M1 ferrox-server template.
- `proto/soter.v1.proto` — the wire contract (`Check` + `Identity` RPCs,
  `SmtQuery` / `SmtReport` / `SolverIdentity` / `SmtStatusCode`).
- `Dockerfile` (self-contained Stage 1 CVC5 + Stage 2 Rust + Stage 3
  minimal runtime), `docker-compose.yml` (local dev, port 50052),
  `ops/cloudbuild.prod.yaml`, `ops/cloudrun.prod.yaml`, `ops/iam-setup.sh`,
  `ops/smoke.sh`.

### Notes

- v1 production deploy: `ingress=internal`, SA
  `soter-server@reflective-labs.iam.gserviceaccount.com`. See
  `kb/Architecture/Cloud Run Deployment.md` for live state once shipped.
- Publish to crates.io is a separate ticket — local path patch unblocks
  marquee-app M3 work in the meantime.

## [0.2.2] - 2026-05-17

### Changed

- Bump `converge-pack` to `3.9.1`.
- Back-port the 5-gate `release-check` recipe (security-audit, coverage,
  performance-profile, soak, lint, test) plus `coverage` / `performance-profile`
  / `soak` recipes from the converge-extension template. First clean
  `just release-check` run.

## [0.2.1] - 2026-05-15

### Changed

- Bump `converge-pack` to `3.9.0`.

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
