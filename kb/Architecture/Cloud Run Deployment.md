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

## Deployed state (M2 shipped 2026-06-16)

| Field | Value |
|---|---|
| Project | `reflective-labs` (number 640630843925) |
| Region | `europe-west1` |
| Service URL | `https://soter-server-640630843925.europe-west1.run.app` |
| Ingress | `internal` |
| VPC connector | `solver-egress-ew1` (shared with ferrox-server) |
| Service account | `soter-server@reflective-labs.iam.gserviceaccount.com` |
| Image tag | `v0.3.0-6ee2f3a` |
| Image registry | `europe-west1-docker.pkg.dev/reflective-labs/converge/soter-server` |
| Concurrency | 1 (CVC5 single-process per query) |
| Min / Max instances | 0 / 10 (cold-start acceptable per spec §10.4) |
| CPU / Memory | 1 vCPU / 2 GiB |
| Cloud Run timeout | 60s (headroom over the 30s spec SMT budget) |
| Bearer auth | off (SOTER_AUTH_TOKEN unset; tenant header is the gate) |
| Tenant allowlist | `quorum-sense` (8 in-flight) |
| Health check | `grpc.health.v1.Health` via `tonic-health` |
| Reflection | `grpc.reflection.v1.ServerReflection` via `tonic-reflection` |
| Smoke script | `ops/smoke.sh <url>` |
| Cloud Build SHA | `30e5f3ae-54eb-4d87-a6cd-bbe44a89fe30` (8m49s — partial layer cache hit) |
| Workspace version | `converge-soter-smt = 0.3.0` (RemoteSmtBackend behind `remote` feature) |

**Smoke verified 2026-06-16 — 5/5 checks:**
- `grpc.health.v1.Health/Check` → `SERVING`
- `grpcurl list` → `soter.v1.SoterSolver`, `grpc.health.v1.Health`, `grpc.reflection.v1.ServerReflection`
- `SoterSolver/Check` without `x-converge-app` → `INVALID_ARGUMENT: missing x-converge-app header`
- `SoterSolver/Check` with unknown tenant → `PERMISSION_DENIED: unknown tenant: <slug>`
- `SoterSolver/Check` with `x-converge-app: quorum-sense` + trivial `(check-sat)` → `status: SMT_STATUS_SAT`, `solver: "cvc5"`, `nativeVersion: "1.3.3"`, query_hash present (`sha256:...`), wall_time ~101ms

**Cloud Build journey (4 attempts, all real defects):**
1. CVC5 cmake failed — Debian's `python3` lacks `venv`. Fixed by adding `python3-venv`.
2. cargo couldn't resolve `organism-adversarial` — Dockerfile only cloned `bedrock-platform/converge`, not `bedrock-platform/organism`. Fixed.
3. protoc missing `google/protobuf/empty.proto` — Debian's `protobuf-compiler` doesn't expose well-known types on the default search path. Fixed by dropping the import and using an inline `EmptyRequest`.
4. SUCCESS.

**Smoke connectivity:** ran via temporary `--ingress=all` flip from the dev laptop with a Google ID token (auth still required throughout). Restored to `--ingress=internal` immediately after. Cloud Shell is the canonical smoke runner once the user is at a terminal that has Cloud Shell access — see `ops/smoke.sh` header.

## Unblocked

**M3** — quorum-sense flips `cvc5` cfg gates to `remote-smt`. Spec §8 M3 has the
exact code-site list. Soter-smt's 0.3.0 workspace bump is in; publish to
crates.io is a separate ticket (M3 can use `[patch.crates-io]` path patch in
the meantime).
