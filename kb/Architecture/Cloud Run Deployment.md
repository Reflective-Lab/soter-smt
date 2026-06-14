---
source: llm
type: architecture-note
date: 2026-06-14
relates_to:
  - ../../crates/soter/src/backend.rs
  - ../../crates/cvc5-sys/
spec: marquee-apps/quorum-sense/docs/superpowers/specs/2026-06-14-converge-grpc-suggestor-pattern-design.md
---

# Cloud Run Deployment

Pointer note. Authoritative spec lives in quorum-sense (first consumer):
`marquee-apps/quorum-sense/docs/superpowers/specs/2026-06-14-converge-grpc-suggestor-pattern-design.md`.

## soter-smt's role

soter-smt becomes the **second extension** to follow the gRPC suggestor pattern.
M2 of the spec is entirely soter-smt work:

- Add `proto/soter.v1.proto` — `SoterSolver/Check` + `Identity` RPCs (§4.2 of
  spec).
- Add `crates/soter-server/` — tonic server wrapping `Cvc5FfiBackend`
  internally. Same interceptor stack as ferrox-server (tenant allowlist,
  optional `SOTER_AUTH_TOKEN` bearer, `grpc.health.v1.Health`).
- Add `Dockerfile` at repo root — self-contained, Stage 1 builds CVC5 from
  source pinned to `cvc5-sys`'s version; Stage 2 compiles Rust server. **Do
  not** consume `runtime-runway/docker/Dockerfile.math-base` (per §3.5 Rule 1).
- Add `docker-compose.yml` mirroring ferrox-solvers' pattern for local dev.
- Add `crates/soter/src/remote.rs` — `RemoteSmtBackend` (client) behind a new
  `remote` Cargo feature. Implements `SmtBackend::solve` over gRPC.
- Publish `converge-soter-smt` **0.3.0** with the `remote` feature.

## Invariants the pattern enforces

- `Cvc5FfiBackend` stays server-side only — marquee-apps never link
  `cvc5-sys`.
- `ScriptedSmtBackend` (already exists, gated behind `fake-backend`) remains
  test-only. Not a production fallback. Not a local-dev fallback when
  `remote-smt` is the consumer's feature.
- The `remote` Cargo feature in `converge-soter-smt` and the `remote-smt` Cargo
  feature in marquee-apps are mutually exclusive with `cvc5`.

## Downstream

M3 = quorum-sense flips to `RemoteSmtBackend`. Spec §8 M3 lists exact code-site
changes. No work in soter-smt for M3 beyond the 0.3.0 publish.
