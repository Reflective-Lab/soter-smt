---
tags: [planning, roadmap]
source: mixed
date: 2026-05-14
---
# Milestones

Soter is pulled by the Arbiter high-risk invariant work. The point is not a
generic SMT playground; the point is selected counterexample search for policy
claims that are too important to leave as ordinary LLM arguments.

## Shipped 2026-05-17 — v0.2.2 (Converge 3.9.1 alignment)

- Bump `converge-pack` to `3.9.1`.
- Back-port the 5-gate `release-check` recipe (security-audit, coverage,
  performance-profile, soak, lint, test) plus `coverage`,
  `performance-profile`, `soak` recipes from the converge-extension template.
- First clean `just release-check` run.
- Tag v0.2.2 and publish `converge-soter-cvc5-sys` + `converge-soter-smt`.

## Completed 2026-05-14

- Created the `soter-smt` extension scaffold.
- Added safe SMT query/report/backend/suggestor types.
- Added typed `soter` provenance at the proposal boundary.
- Added Formation capability descriptors under `soter.smt`.
- Added native CVC5 sys crate and pinned source-build recipe.
- Added a native CVC5 version-smoke test behind the `cvc5` feature.
- Defaulted the source build to `--no-poly` for a portable first CVC5 FFI path.
- Added a narrow CVC5 solving API for SMT-LIB `check-sat` payloads.
- Added native SAT, UNSAT, and invalid-SMT-LIB tests behind the `cvc5` feature.
- Added the first typed abstract Arbiter conditional invariant fixture.
- Added property tests for deterministic invariant rendering and invalid
  threshold rejection.
- Added native CVC5 tests showing strict Arbiter model = UNSAT and broken model
  = SAT with a counterexample.
- Documented CVC5 FFI, evidence semantics, and the Arbiter-first pull.

## Completed 2026-05-15

- Migrated SMT report execution metadata to Converge's shared
  `ExecutionIdentity` contract.
- Recorded CVC5 native solver identity in SMT evidence, including linked
  version/build identity, source mode, expected and actual checkout commit, and
  runtime query config.
- Made `execution_identity` required on `SmtReport` so replayed/audited
  evidence cannot silently fall back to an unknown solver identity.

## Short-Term

- Native CVC5 assurance hardening:

  Current state: every `SmtReport` carries required Converge
  `ExecutionIdentity`, and CVC5-backed reports include linked version, source
  mode, expected and actual checkout commit, build identity, and runtime query
  config. The remaining work is to make the native dependency path reproducible
  and enforceable in CI.

  Why this matters:
  - **Policy assurance perspective:** Soter evidence supports high-risk policy
    claims; a result is weaker if the native CVC5 build is ambiguous.
  - **Audit perspective:** reviewers need to distinguish "same SMT-LIB query,
    same CVC5 bits" from "same query, different native binary or flags."
  - **Release/CI perspective:** checkout or build-flag drift should fail in the
    assurance lane, not be discovered during replay.
  - **Developer perspective:** `SOTER_CVC5_ROOT` and CVC5 auto-download are
    acceptable local conveniences, but they must not silently define trusted
    assurance inputs.

  1. Add a checked-in CVC5 native dependency lock/audit manifest with name,
     version, source URL, expected checkout commit, configure flags, and
     available artifact/header/library fingerprints.
  2. Add Linux and macOS CI coverage for CVC5-feature check, clippy, and tests,
     failing when the native checkout commit differs from the manifest.
  3. Tighten `SOTER_CVC5_ROOT` so assurance CI either rejects external roots or
     requires an explicit trusted-external-root mode that records and
     version-checks the linked CVC5.
  4. Disable or isolate CVC5 auto-download in assurance CI. Local
     auto-download remains a convenience path only.

## Deferred

- Additional mandatory CI lanes beyond CVC5 assurance. Re-open when a product
  requires them.
- Other SMT backends. Re-open when an app needs them.
- Proof-checking integration. Re-open when a product requires independently
  checked proof artifacts.
