---
tags: [planning, roadmap]
source: mixed
date: 2026-05-14
---
# Milestones

Soter is pulled by the Arbiter high-risk invariant work. The point is not a
generic SMT playground; the point is selected counterexample search for policy
claims that are too important to leave as ordinary LLM arguments.

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
- Documented CVC5 FFI, evidence semantics, and the Arbiter-first pull.

## Short-Term

- Add the first Arbiter conditional invariant query fixture.
- Add nightly/manual real-CVC5 CI after native linking is stable.

## Deferred

- Required real-CVC5 CI for every PR. Re-open when conditional invariant
  queries encode actual Arbiter claims.
- Other SMT backends. Re-open when an app needs them.
- Proof-checking integration. Re-open when a product requires independently
  checked proof artifacts.
