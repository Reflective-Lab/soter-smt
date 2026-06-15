# M2 — soter-server reaches Cloud Run prod

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended for Phases A/B/C/E) or superpowers:executing-plans. Phase D (Cloud Build + Cloud Run) is GCP-touching — inline execution with a human in the loop.

**Goal:** Stand up `soter-server`, a new gRPC service that wraps `Cvc5FfiBackend`, in GCP project `reflective-labs`. Publish `converge-soter-smt 0.3.0` with a `remote` feature exposing `RemoteSmtBackend` (gRPC client implementing `SmtBackend`). After M2, marquee-apps can swap `cvc5` FFI for a remote SMT call; no marquee-app code changes yet — that's M3.

**Architecture:** This is the **second instance** of the gRPC suggestor pattern (`marquee-apps/quorum-sense/docs/superpowers/specs/2026-06-14-converge-grpc-suggestor-pattern-design.md` §3, §5.2, §5.3). M1 (`ferrox-server`) shipped 2026-06-15 and proved the pipeline end-to-end. M2 copies the proven mechanics (tenant infra, health, reflection, JSON logs, Cloud Build, Cloud Run, smoke) and adapts them to soter-smt's shape: new proto (`soter.v1`), new server crate, new self-contained Dockerfile with **CVC5** instead of OR-Tools/HiGHS in Stage 1, and a new client crate (`RemoteSmtBackend`).

**Tech Stack:** Rust 1.96 (workspace pin), edition 2024, `tonic 0.14`, `tonic-prost 0.14`, `tonic-health 0.14`, `tonic-reflection 0.14`, `prost 0.14`, `tokio`, `tracing` + `tracing-subscriber` (`json`), `uuid v4`, `serial_test 3`, `temp-env 0.3`. CVC5 `cvc5-1.3.3` (commit `8ff882e3e42f046867d2ac2e33e92b3d026144ae`), built `--no-poly --auto-download`. Cloud Build, Cloud Run, Artifact Registry, gcloud CLI.

**Reference plan:** `mosaic-extensions/ferrox-solvers/docs/superpowers/plans/2026-06-14-m1-ferrox-server-cloud-run.md`. Where soter-server's task is mechanically identical to ferrox-server's task, this plan says **"mirror M1.X"** and lists only the substitutions, not the full code. Tasks where soter genuinely diverges (proto, CVC5 Dockerfile, client crate) are spelled out in full.

**GCP details (shared with M1, no setup needed):**
- Project ID: `reflective-labs` (number 640630843925)
- Region: `europe-west1`
- Artifact Registry repo: `europe-west1-docker.pkg.dev/reflective-labs/converge` (exists from M1)
- VPC connector: `solver-egress-ew1` (10.8.0.0/28, 2× e2-micro, READY from M1)
- Required APIs: run, cloudbuild, artifactregistry, vpcaccess (all enabled from M1)
- **New per-extension SA:** `soter-server@reflective-labs.iam.gserviceaccount.com` (created in D1)

**Spec sections this plan implements:** §3.5 (pattern rules), §4.2 (Soter proto), §4.3 (metadata headers), §5.2 (soter-server), §5.3 (RemoteSmtBackend), §6 (tenant allowlist), §7 (network, auth, VPC), §8 M2 (this milestone).

**Out of scope (deferred to follow-on tickets / M3 / future milestones):**
- Per-minute rate limiting (`max_requests_per_minute`) — in-flight cap suffices for v1.
- Bearer auth on the wire (`SOTER_AUTH_TOKEN` env stays unset; tenant header is the only gate).
- Prometheus metrics endpoint.
- Cloud Monitoring dashboard + alerts.
- `cargo publish converge-soter-smt 0.3.0` to crates.io — the M2 plan stops at the workspace tag bump; publish is a separate ticket once the release flow is settled. M3's marquee-app dep flip will use the path patch until the publish lands.

---

## File Structure

**New files (10):**
- `proto/soter.v1.proto` — gRPC contract per spec §4.2.
- `crates/soter-server/Cargo.toml` — new crate manifest.
- `crates/soter-server/build.rs` — tonic-prost-build + descriptor emit.
- `crates/soter-server/src/lib.rs` — exposes `interceptor`, `tenants`.
- `crates/soter-server/src/main.rs` — binary entrypoint.
- `crates/soter-server/src/service.rs` — `SoterSolverService` wrapping `Cvc5FfiBackend`.
- `crates/soter-server/src/convert.rs` — proto ↔ `SmtQuery` / `SmtReport` mapping.
- `crates/soter-server/src/tenants.rs` — verbatim from ferrox-server tenants.rs, slug list adapted.
- `crates/soter-server/src/interceptor.rs` — verbatim from ferrox-server interceptor.rs.
- `crates/soter-server/tests/tenant_registry.rs` — verbatim from ferrox-server.
- `crates/soter/src/remote.rs` — `RemoteSmtBackend` client.
- `Dockerfile` — Stage 1 CVC5 build, Stage 2 Rust compile, Stage 3 runtime.
- `docker-compose.yml` — local dev (mirror ferrox-solvers/docker-compose.yml).
- `ops/iam-setup.sh` — soter-server SA + IAM (mirror M1.D1).
- `ops/cloudbuild.prod.yaml` — Cloud Build (mirror M1.D2; image name = `soter-server`).
- `ops/cloudrun.prod.yaml` — Cloud Run manifest (mirror M1.D3; service name = `soter-server`).
- `ops/smoke.sh` — grpcurl smoke (mirror M1.E1; SoterSolver/Check + Identity).

**Modified files (5):**
- `Cargo.toml` (workspace) — add `soter-server` member; add server-side dev/runtime deps.
- `crates/soter/Cargo.toml` — add `remote` Cargo feature; add `tonic`, `prost`, `tonic-prost-build` (build-dep) when `remote` is on.
- `crates/soter/src/lib.rs` — `pub use remote::RemoteSmtBackend;` behind `#[cfg(feature = "remote")]`.
- `Justfile` — add `deploy-build`, `deploy-apply`, `smoke-prod`, `tenants-show`.
- `kb/Architecture/Cloud Run Deployment.md` — record deployed state on M2 ship.

**Cross-repo touched on M2 ship:**
- `marquee-apps/quorum-sense/docs/superpowers/specs/2026-06-14-converge-grpc-suggestor-pattern-design.md` — flip "### M2 — soter-server reaches Cloud Run prod" to "✅ shipped YYYY-MM-DD" (M2.F2).

---

## Phase A — soter-server crate, proto, tenant infrastructure

In-process work. No deploy, no native deps required (most Phase A tasks build with `--no-default-features`).

### Task A1: Write `proto/soter.v1.proto`

**Files:**
- Create: `proto/soter.v1.proto`

Per spec §4.2.

- [ ] **Step 1:** Create `proto/soter.v1.proto`:

```protobuf
syntax = "proto3";

package soter.v1;

import "google/protobuf/empty.proto";

option java_package    = "zone.soter.v1";
option go_package      = "github.com/reflective-lab/soter/soterv1";

message SmtQuery {
  string query_id            = 1;
  string smtlib              = 2;  // SMT-LIB v2 input
  uint64 timeout_ms          = 3;
  bool   produce_model       = 4;
  bool   produce_unsat_core  = 5;
}

message SolverIdentity {
  string solver              = 1;
  string native_backend      = 2;  // "cvc5"
  string native_version      = 3;
  string rust_crate          = 4;
  string rust_crate_version  = 5;
}

enum SmtStatusCode {
  SMT_STATUS_UNSPECIFIED = 0;
  SMT_STATUS_SAT         = 1;
  SMT_STATUS_UNSAT       = 2;
  SMT_STATUS_UNKNOWN     = 3;
  SMT_STATUS_TIMEOUT     = 4;
  SMT_STATUS_ERROR       = 5;
}

message SmtReport {
  string         query_id           = 1;
  string         query_hash         = 2;
  SmtStatusCode  status             = 3;
  SolverIdentity solver_identity    = 4;
  optional string model             = 5;  // populated when status=SAT and produce_model
  optional string unsat_core        = 6;  // populated when status=UNSAT and produce_unsat_core
  optional string diagnostics       = 7;
  double         wall_time_seconds  = 8;
}

message CheckRequest {
  SmtQuery query = 1;
}

service SoterSolver {
  rpc Check(CheckRequest) returns (SmtReport);
  rpc Identity(google.protobuf.Empty) returns (SolverIdentity);
}
```

- [ ] **Step 2: Commit**

```bash
cd mosaic-extensions/soter-smt
mkdir -p proto
git add proto/soter.v1.proto
git commit -m "$(cat <<'EOF'
feat(proto): add soter.v1 contract (M2.A1)

SoterSolver service with Check + Identity RPCs. SmtStatusCode is a typed
enum (cleaner than the spec's string status; covers spec §4.2 intent and
makes gRPC clients tighter). query_hash is server-side: client passes
the SMT-LIB; server hashes for telemetry / dedup-friendly logging.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task A2: Bootstrap `crates/soter-server/` (Cargo.toml, build.rs, empty src)

**Files:**
- Create: `crates/soter-server/Cargo.toml`
- Create: `crates/soter-server/build.rs`
- Create: `crates/soter-server/src/main.rs` (stub)
- Create: `crates/soter-server/src/lib.rs` (stub)
- Create: `crates/soter-server/src/service.rs` (stub)
- Create: `crates/soter-server/src/convert.rs` (stub)
- Modify: `Cargo.toml` (workspace) — add `soter-server` member

- [ ] **Step 1: Write `crates/soter-server/Cargo.toml`**

```toml
[package]
name          = "converge-soter-server"
version.workspace    = true
edition.workspace    = true
rust-version.workspace = true
license.workspace    = true
repository.workspace = true
publish       = false
description   = "gRPC server exposing the soter SMT solver (CVC5)"

[lib]
name = "converge_soter_server"
path = "src/lib.rs"

[[bin]]
name = "soter-server"
path = "src/main.rs"

[features]
default = ["full"]
full    = ["dep:soter-cvc5-sys", "soter/cvc5", "soter-cvc5-sys/link"]

[dependencies]
soter                = { workspace = true }
soter-cvc5-sys       = { workspace = true, optional = true }

tonic                = { version = "0.14", features = ["tls-ring"] }
tonic-prost          = "0.14"
tonic-health         = "0.14"
tonic-reflection     = "0.14"
prost                = "0.14"
tokio                = { workspace = true }
tracing              = { workspace = true }
tracing-subscriber   = { version = "0.3.23", features = ["env-filter", "json"] }
anyhow               = "1"
uuid                 = { version = "1", features = ["v4"] }
sha2                 = { workspace = true }

[build-dependencies]
tonic-prost-build = "0.14"

[dev-dependencies]
serial_test = "3"
temp-env    = "0.3"

[lints]
workspace = true
```

- [ ] **Step 2: Write `crates/soter-server/build.rs`** (mirror M1.D2 build.rs + descriptor emit + cvc5-sys rpath propagation)

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let proto_dir = manifest_dir.join("../../proto");
    let proto = proto_dir.join("soter.v1.proto");

    println!("cargo:rerun-if-changed={}", proto.display());

    // Propagate rpath from cvc5-sys so the runtime binary finds libcvc5
    // alongside libtonic etc. cvc5-sys's links="cvc5" emits DEP_CVC5_LIB_DIR.
    if let Ok(lib_dir) = std::env::var("DEP_CVC5_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }

    // Emit the encoded FileDescriptorSet for tonic-reflection.
    let descriptor_path = std::path::PathBuf::from(std::env::var("OUT_DIR")?)
        .join("soter_descriptor.bin");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(&[proto], &[proto_dir])?;

    Ok(())
}
```

- [ ] **Step 3: Stub `crates/soter-server/src/lib.rs`**

```rust
//! Public surface of `converge-soter-server` for integration tests + future
//! library consumers. The binary entrypoint stays in `main.rs`.

pub mod interceptor;
pub mod tenants;
```

(Modules are created in A3.)

- [ ] **Step 4: Stub `crates/soter-server/src/main.rs`** — just enough to compile

```rust
mod convert;
mod service;

pub mod proto {
    pub mod soter {
        #[allow(
            clippy::doc_markdown,
            clippy::default_trait_access,
            clippy::too_many_lines,
            clippy::large_enum_variant
        )]
        pub mod v1 {
            tonic::include_proto!("soter.v1");
        }
    }
}

fn main() {
    eprintln!("soter-server stub — wire main() in A4/B2");
    std::process::exit(1);
}
```

- [ ] **Step 5: Stub `crates/soter-server/src/service.rs` and `crates/soter-server/src/convert.rs`**

`src/service.rs`:

```rust
//! gRPC service implementation. Wraps `Cvc5FfiBackend`. Filled in A5/A6.
```

`src/convert.rs`:

```rust
//! Proto ↔ `SmtQuery` / `SmtReport` mapping. Filled in A5.
```

- [ ] **Step 6: Add to workspace Cargo.toml**

In `mosaic-extensions/soter-smt/Cargo.toml`, change:

```toml
members = [
    "crates/cvc5-sys",
    "crates/soter",
]
```

to:

```toml
members = [
    "crates/cvc5-sys",
    "crates/soter",
    "crates/soter-server",
]
```

- [ ] **Step 7: Build verification**

Run: `cd mosaic-extensions/soter-smt && cargo check -p converge-soter-server --no-default-features`

(Same reasoning as M1: `--no-default-features` avoids triggering the CVC5 native link before vendor/cvc5 is built. For dev runs, `make cvc5` first then `cargo check -p converge-soter-server` with default features.)

Expected: clean. Stub binary compiles.

- [ ] **Step 8: Commit**

```bash
cd mosaic-extensions/soter-smt
git add Cargo.toml crates/soter-server/
git commit -m "$(cat <<'EOF'
feat(soter-server): scaffold gRPC server crate (M2.A2)

Bootstrap converge-soter-server: Cargo.toml + build.rs (tonic-prost-build
with descriptor emit for tonic-reflection, cvc5-sys rpath propagation),
plus stub lib.rs / main.rs / service.rs / convert.rs. Mirrors the
ferrox-server crate layout exactly so the next tasks copy verbatim.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task A3: Tenant allowlist + registry (mirror M1.A1 + M1.A1-fixup)

**Files:**
- Create: `crates/soter-server/src/tenants.rs`

The file is identical to ferrox-server's `tenants.rs` after the M1.A1-fixup. Copy verbatim with two adjustments:

1. The doc comment top-block stays the same.
2. The `TENANTS` const seed list: same one entry for v1.

```rust
//! Tenant allowlist and per-tenant in-flight semaphore.
//!
//! Per design spec §6 (`marquee-apps/quorum-sense/docs/superpowers/specs/
//! 2026-06-14-converge-grpc-suggestor-pattern-design.md`):
//! - Allowlist is compile-time `const TENANTS`.
//! - Adding a tenant requires a server image rebuild and redeploy.
//! - Per-tenant in-flight cap; over-limit returns `RESOURCE_EXHAUSTED`.
//! - Unknown tenant returns `PERMISSION_DENIED`.
//! - Missing `x-converge-app` header returns `INVALID_ARGUMENT`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tonic::Status;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tenant {
    pub slug: &'static str,
    pub max_in_flight: u32,
}

impl Tenant {
    #[must_use]
    pub fn cap_for(slug: &str) -> Option<u32> {
        TENANTS.iter().find(|t| t.slug == slug).map(|t| t.max_in_flight)
    }
}

pub const TENANTS: &[Tenant] = &[
    Tenant { slug: "quorum-sense", max_in_flight: 8 },
];

#[derive(Clone, Copy, Debug)]
pub struct TenantSlug(pub &'static str);

#[derive(Debug)]
pub struct TenantRegistry {
    permits: HashMap<&'static str, Arc<Semaphore>>,
}

impl TenantRegistry {
    #[must_use]
    pub fn from_const() -> Self {
        let permits = TENANTS
            .iter()
            .map(|t| (t.slug, Arc::new(Semaphore::new(t.max_in_flight as usize))))
            .collect();
        Self { permits }
    }

    #[must_use]
    pub fn lookup(slug: &str) -> Option<&'static Tenant> {
        TENANTS.iter().find(|t| t.slug == slug)
    }

    /// Kept async to preserve the API shape if we later switch to
    /// `acquire_owned().await` (blocking) for backpressure semantics.
    #[allow(clippy::unused_async)]
    pub async fn acquire(&self, slug: &str) -> Result<OwnedSemaphorePermit, Status> {
        let sem = self
            .permits
            .get(slug)
            .ok_or_else(|| Status::permission_denied(format!("unknown tenant: {slug}")))?
            .clone();
        sem.try_acquire_owned().map_err(|e| match e {
            tokio::sync::TryAcquireError::NoPermits => {
                Status::resource_exhausted(format!("tenant {slug} at in-flight cap"))
            }
            tokio::sync::TryAcquireError::Closed => {
                Status::unavailable(format!("tenant {slug} semaphore closed"))
            }
        })
    }
}

impl Default for TenantRegistry {
    fn default() -> Self {
        Self::from_const()
    }
}
```

**Why `max_in_flight: 8` for soter** (vs. 4 for ferrox): SMT checks are typically cheaper than CP-SAT solves; quorum's formation cycle issues multiple parallel reachability + synthesis queries.

- [ ] **Step 1: Write the file** (content above).

- [ ] **Step 2: Build**: `cargo check -p converge-soter-server --no-default-features` — clean.

- [ ] **Step 3: Commit**

```bash
git add crates/soter-server/src/tenants.rs
git commit -m "$(cat <<'EOF'
feat(soter-server): add tenant allowlist + runtime registry (M2.A3)

Compile-time TENANTS table seeds an in-process Semaphore-per-tenant map.
Acquire returns typed gRPC Status. Quorum-sense seeded with 8 in-flight
(higher than ferrox's 4 — SMT queries are cheaper and formation cycles
fan out parallel reachability+synthesis checks).

Mirrors mosaic-extensions/ferrox-solvers/crates/ferrox-server/src/tenants.rs
post M1.A1-fixup.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task A4: Combined `request_interceptor` (mirror M1.A3 + M1.C2)

**Files:**
- Create: `crates/soter-server/src/interceptor.rs`

Content identical to ferrox-server's `interceptor.rs` after M1.C2 (the version that also mints/accepts `x-request-id`). Substitute `FERROX_AUTH_TOKEN` → `SOTER_AUTH_TOKEN`. The `crate::tenants::{TenantRegistry, TenantSlug}` import resolves correctly because tenants is exposed via lib.rs.

```rust
//! Tonic interceptor: validate `Authorization` (optional bearer) and
//! `x-converge-app` (tenant); mint/accept `x-request-id`; attach both to
//! request extensions so the service layer can use them in spans.
//!
//! Per spec §6 and §7.2 — bearer is optional (gated by `SOTER_AUTH_TOKEN`
//! env presence); tenant header is required.

use tonic::{Request, Status};
use uuid::Uuid;

use crate::tenants::{TenantRegistry, TenantSlug};

#[derive(Clone, Debug)]
pub struct RequestId(pub String);

#[allow(clippy::result_large_err)]
pub fn request_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
    // Bearer (optional — disabled when env unset).
    if let Ok(expected) = std::env::var("SOTER_AUTH_TOKEN") {
        let provided = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != format!("Bearer {expected}") {
            return Err(Status::unauthenticated("invalid or missing token"));
        }
    }

    // Tenant (required).
    let slug = req
        .metadata()
        .get("x-converge-app")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Status::invalid_argument("missing x-converge-app header"))?;

    let tenant = TenantRegistry::lookup(slug)
        .ok_or_else(|| Status::permission_denied(format!("unknown tenant: {slug}")))?;

    // Request ID — accept if present, mint UUIDv4 otherwise.
    let request_id = req
        .metadata()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.extensions_mut().insert(TenantSlug(tenant.slug));
    req.extensions_mut().insert(RequestId(request_id));
    Ok(req)
}
```

- [ ] **Step 1: Write the file.**
- [ ] **Step 2: Build**: `cargo check -p converge-soter-server --no-default-features` — clean.
- [ ] **Step 3: Commit:**

```bash
git add crates/soter-server/src/interceptor.rs
git commit -m "$(cat <<'EOF'
feat(soter-server): combined bearer + tenant + request_id interceptor (M2.A4)

Mirrors mosaic-extensions/ferrox-solvers/crates/ferrox-server/src/interceptor.rs
post M1.C2. Bearer is SOTER_AUTH_TOKEN-gated (off in v1); x-converge-app
is required; x-request-id is minted via Uuid::new_v4 when absent. Both
TenantSlug and RequestId attached to request extensions for the service
layer to consume in tracing spans.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task A5: Wire `convert.rs` and `service.rs` — Check + Identity RPCs over `Cvc5FfiBackend`

**Files:**
- Modify: `crates/soter-server/src/convert.rs`
- Modify: `crates/soter-server/src/service.rs`

This is where soter genuinely diverges from ferrox. Two RPCs (`Check`, `Identity`) instead of three (`SolveCp`, `SolveLp`, `SolveMip`). The service holds an `Arc<dyn SmtBackend>` rather than constructing the solver per-call.

- [ ] **Step 1: Write `crates/soter-server/src/convert.rs`**

```rust
//! Proto ↔ `soter::SmtQuery` / `soter::SmtReport` mapping.

use sha2::{Digest, Sha256};
use soter::{SmtError, SmtQuery, SmtReport, SmtStatus};
use tonic::Status;

use crate::proto::soter::v1 as pb;

pub fn smt_query_from_proto(req: pb::CheckRequest) -> Result<SmtQuery, Status> {
    let Some(q) = req.query else {
        return Err(Status::invalid_argument("CheckRequest.query is required"));
    };

    if q.smtlib.trim().is_empty() {
        return Err(Status::invalid_argument("SmtQuery.smtlib must be non-empty"));
    }
    if q.query_id.trim().is_empty() {
        return Err(Status::invalid_argument("SmtQuery.query_id must be non-empty"));
    }

    Ok(SmtQuery {
        id: q.query_id,
        smtlib: q.smtlib,
        timeout: std::time::Duration::from_millis(q.timeout_ms),
        produce_model: q.produce_model,
        produce_unsat_core: q.produce_unsat_core,
    })
}

pub fn smt_report_to_proto(query_id: String, smtlib_hash: String, report: SmtReport) -> pb::SmtReport {
    let status = match report.status {
        SmtStatus::Sat => pb::SmtStatusCode::Sat,
        SmtStatus::Unsat => pb::SmtStatusCode::Unsat,
        SmtStatus::Unknown => pb::SmtStatusCode::Unknown,
        SmtStatus::Timeout => pb::SmtStatusCode::Timeout,
        SmtStatus::Error => pb::SmtStatusCode::Error,
    };
    pb::SmtReport {
        query_id,
        query_hash: smtlib_hash,
        status: status as i32,
        solver_identity: Some(pb::SolverIdentity {
            solver: report.solver.clone(),
            native_backend: "cvc5".to_string(),
            native_version: env!("CARGO_PKG_VERSION").to_string(),
            rust_crate: "converge-soter-smt".to_string(),
            rust_crate_version: env!("CARGO_PKG_VERSION").to_string(),
        }),
        model: report.model,
        unsat_core: report.unsat_core,
        diagnostics: report.diagnostics,
        wall_time_seconds: report.wall_time_seconds,
    }
}

pub fn smt_error_to_status(err: SmtError) -> Status {
    // Map soter's typed error to gRPC status.
    Status::internal(format!("smt backend error: {err}"))
}

pub fn smtlib_hash(smtlib: &str) -> String {
    let mut h = Sha256::new();
    h.update(smtlib.as_bytes());
    let digest = h.finalize();
    // First 12 bytes as hex — 24 chars, enough for log dedup.
    digest.iter().take(12).fold(String::with_capacity(24), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}
```

Note: This assumes `soter::SmtQuery` has fields `id`, `smtlib`, `timeout` (Duration), `produce_model`, `produce_unsat_core`, and `SmtReport` has `status`, `solver`, `model`, `unsat_core`, `diagnostics`, `wall_time_seconds`. Verify against `crates/soter/src/types.rs` and adjust field names if the types crate uses different ones. The conversion may need `?Sized`-style adaptation if `SmtQuery::new(...)` is the constructor rather than direct struct literal.

- [ ] **Step 2: Write `crates/soter-server/src/service.rs`**

```rust
use std::sync::Arc;

use tonic::{Request, Response, Status};

use soter::SmtBackend;
use converge_soter_server::interceptor::RequestId;
use converge_soter_server::tenants::{TenantRegistry, TenantSlug};

use crate::convert::{smt_error_to_status, smt_query_from_proto, smt_report_to_proto, smtlib_hash};
use crate::proto::soter::v1::soter_solver_server::SoterSolver;
use crate::proto::soter::v1::{CheckRequest, SmtReport as PbSmtReport, SolverIdentity};

#[derive(Clone)]
pub struct SoterSolverService {
    backend: Arc<dyn SmtBackend>,
    tenants: Arc<TenantRegistry>,
}

impl SoterSolverService {
    pub fn new(backend: Arc<dyn SmtBackend>, tenants: Arc<TenantRegistry>) -> Self {
        Self { backend, tenants }
    }
}

struct RequestContext<'a> {
    tenant: &'static str,
    request_id: &'a str,
}

fn request_context<R>(req: &Request<R>) -> Result<RequestContext<'_>, Status> {
    let tenant = req
        .extensions()
        .get::<TenantSlug>()
        .map(|t| t.0)
        .ok_or_else(|| Status::internal("missing TenantSlug extension"))?;
    let request_id = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.as_str())
        .ok_or_else(|| Status::internal("missing RequestId extension"))?;
    Ok(RequestContext { tenant, request_id })
}

#[tonic::async_trait]
impl SoterSolver for SoterSolverService {
    async fn check(
        &self,
        request: Request<CheckRequest>,
    ) -> Result<Response<PbSmtReport>, Status> {
        let ctx = request_context(&request)?;
        let span = tracing::info_span!(
            "check",
            tenant_app = %ctx.tenant,
            request_id = %ctx.request_id,
            rpc_method = "soter.v1.SoterSolver/Check",
        );
        let started = std::time::Instant::now();
        let tenant_slug = ctx.tenant;

        let _tenant_permit = self.tenants.acquire(tenant_slug).await?;

        let inner = request.into_inner();
        let query = smt_query_from_proto(inner)?;
        let query_id = query.id.clone();
        let smtlib_h = smtlib_hash(&query.smtlib);

        let report = self.backend.solve(&query).await.map_err(smt_error_to_status);
        let elapsed_us = started.elapsed().as_micros();

        match report {
            Ok(r) => {
                span.in_scope(|| {
                    tracing::info!(
                        solve_duration_us = elapsed_us,
                        status = "ok",
                        smtlib_hash = %smtlib_h,
                        "check completed"
                    );
                });
                Ok(Response::new(smt_report_to_proto(query_id, smtlib_h, r)))
            }
            Err(err) => {
                span.in_scope(|| {
                    tracing::warn!(
                        solve_duration_us = elapsed_us,
                        status = "error",
                        smtlib_hash = %smtlib_h,
                        code = %err.code(),
                        message = %err.message(),
                        "check failed"
                    );
                });
                Err(err)
            }
        }
    }

    async fn identity(
        &self,
        _request: Request<()>,
    ) -> Result<Response<SolverIdentity>, Status> {
        // Identity does NOT require a tenant header — it's a public capability
        // probe. (Spec §4.2 lists it; spec §6 only gates the solve RPCs.)
        Ok(Response::new(SolverIdentity {
            solver: self.backend.name().to_string(),
            native_backend: "cvc5".to_string(),
            native_version: cvc5_version(),
            rust_crate: "converge-soter-smt".to_string(),
            rust_crate_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}

#[cfg(feature = "full")]
fn cvc5_version() -> String {
    // soter-cvc5-sys exposes CVC5_VERSION as a build-script artifact. If
    // unavailable at this exact API surface, fall back to the workspace pin.
    option_env!("DEP_CVC5_VERSION")
        .map(String::from)
        .unwrap_or_else(|| "cvc5-1.3.3".to_string())
}

#[cfg(not(feature = "full"))]
fn cvc5_version() -> String {
    "unavailable-without-full-feature".to_string()
}
```

**Important:** The `Identity` endpoint is INSIDE the `with_interceptor` chain (set up in B2 main.rs), so the tenant interceptor runs against it too. If we want `Identity` to be unauthenticated/tenant-free, split it onto a separate `add_service` call without the interceptor — or accept that callers must always send `x-converge-app` for any RPC on the FerroxSolver-equivalent service. The simpler choice for v1: require the tenant header for `Identity` too. The cost is small (callers already set the header for `Check`). Update the comment in `Identity` to reflect this if you take the simpler path.

- [ ] **Step 3: Update `crates/soter-server/src/main.rs`** to assemble + serve (mirror M1.main.rs)

Replace the stub `main.rs` body with the full server wiring (mirror `mosaic-extensions/ferrox-solvers/crates/ferrox-server/src/main.rs` post-M1, substituting `Ferrox` → `Soter`, `ferrox` → `soter`, `SOTER_AUTH_TOKEN` env, and constructing `Cvc5FfiBackend` instead of `FerroxSolverService::default()`).

Sketch:

```rust
mod convert;
mod service;

pub mod proto {
    pub mod soter {
        #[allow(
            clippy::doc_markdown,
            clippy::default_trait_access,
            clippy::too_many_lines,
            clippy::large_enum_variant
        )]
        pub mod v1 {
            tonic::include_proto!("soter.v1");
        }
    }
}

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::{Identity, Server, ServerTlsConfig};

use converge_soter_server::interceptor::request_interceptor;
use converge_soter_server::tenants::TenantRegistry;
use proto::soter::v1::soter_solver_server::SoterSolverServer;
use service::SoterSolverService;

const SOTER_FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("soter_descriptor");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "soter_server=info".parse().unwrap()),
        )
        .init();

    let addr: SocketAddr = std::env::var("SOTER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;

    // Construct the backend. With --no-default-features (no CVC5), we'd
    // need ScriptedSmtBackend or similar, but the prod image always builds
    // with --features full, so this is the only code path.
    #[cfg(feature = "full")]
    let backend: Arc<dyn soter::SmtBackend> = Arc::new(soter::Cvc5FfiBackend);

    #[cfg(not(feature = "full"))]
    let backend: Arc<dyn soter::SmtBackend> = {
        compile_error!(
            "soter-server requires --features full (CVC5 linked); see Cargo.toml"
        )
    };

    let svc = SoterSolverService::new(backend, Arc::new(TenantRegistry::default()));

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<SoterSolverServer<SoterSolverService>>()
        .await;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(SOTER_FILE_DESCRIPTOR_SET)
        .build_v1()?;

    let cert_path = std::env::var("SOTER_TLS_CERT").unwrap_or_else(|_| "/tls/server.crt".into());
    let key_path = std::env::var("SOTER_TLS_KEY").unwrap_or_else(|_| "/tls/server.key".into());

    let use_tls =
        std::path::Path::new(&cert_path).exists() && std::path::Path::new(&key_path).exists();

    tracing::info!(addr = %addr, tls = use_tls, "soter-server starting");

    if use_tls {
        let cert = std::fs::read(&cert_path)?;
        let key = std::fs::read(&key_path)?;
        let identity = Identity::from_pem(cert, key);
        let mut tls = ServerTlsConfig::new().identity(identity);

        if let Ok(ca_path) = std::env::var("SOTER_TLS_CA") {
            let ca = std::fs::read(ca_path)?;
            tls = tls.client_ca_root(tonic::transport::Certificate::from_pem(ca));
        }

        Server::builder()
            .tls_config(tls)?
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(SoterSolverServer::with_interceptor(svc, request_interceptor))
            .serve(addr)
            .await?;
    } else {
        tracing::warn!("TLS cert/key not found — starting without TLS (dev/test only)");
        Server::builder()
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(SoterSolverServer::with_interceptor(svc, request_interceptor))
            .serve(addr)
            .await?;
    }

    Ok(())
}
```

- [ ] **Step 4: Build with default features**

Run `make cvc5` first if `vendor/cvc5/build/install` doesn't exist. Then:

```bash
cd mosaic-extensions/soter-smt
cargo check -p converge-soter-server
```

Expected: clean. (The native cvc5 link runs.)

- [ ] **Step 5: Commit**

```bash
git add crates/soter-server/src/convert.rs \
        crates/soter-server/src/service.rs \
        crates/soter-server/src/main.rs
git commit -m "$(cat <<'EOF'
feat(soter-server): wire Check + Identity RPCs over Cvc5FfiBackend (M2.A5)

SoterSolverService holds Arc<dyn SmtBackend>. Check acquires per-tenant
permit before invoking the backend, emits info_span with tenant_app /
request_id / rpc_method, logs solve_duration_us + status (ok/error) +
smtlib_hash (sha256 prefix). Identity returns solver name + CVC5 native
version + Rust crate version.

main.rs assembles: tracing_subscriber.json(), tonic-health, tonic-reflection
(soter.v1 + health descriptors), and the interceptor-wrapped service.
Matches M1 ferrox-server's shape.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task A6: Integration tests — tenant registry + interceptor

**Files:**
- Create: `crates/soter-server/tests/tenant_registry.rs`

Copy `mosaic-extensions/ferrox-solvers/crates/ferrox-server/tests/tenant_registry.rs` verbatim. Substitutions:

1. Replace `converge_ferrox_server` → `converge_soter_server` (all imports).
2. Replace `FERROX_AUTH_TOKEN` → `SOTER_AUTH_TOKEN` (in the `temp_env::with_var` calls).
3. The TENANTS test that checks `max_in_flight >= 1` works as-is.
4. The over-cap test uses `Tenant::cap_for("quorum-sense")` — picks up the new cap of 8 automatically.

Run: `cargo test -p converge-soter-server --test tenant_registry` — expect 14/14 passing.

- [ ] **Step 1: Write the test file** (verbatim from ferrox + the two text substitutions above).
- [ ] **Step 2: `cargo test` — 14 pass.**
- [ ] **Step 3: Commit:**

```bash
git add crates/soter-server/tests/tenant_registry.rs
git commit -m "$(cat <<'EOF'
test(soter-server): cover TenantRegistry + request_interceptor (M2.A6)

14 tests mirroring ferrox-server's tenant_registry.rs post-M1.A5/M1.C2-fixup:
5 registry (lookup/acquire/cap), 1 extension contract (TenantSlug round-trip),
6 interceptor (known/missing/unknown tenant, bearer disabled/bad/good),
2 request_id (mint-on-absent, accept-on-present). serial_test + temp-env
keep the env-touching tests deterministic.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase B — Health + reflection (already wired in A5; close-out)

Health + reflection were wired into `main.rs` in A5 to keep the diff coherent. Phase B has no work beyond a verification step.

### Task B1: Local smoke (optional, only if `make cvc5` succeeded)

If `vendor/cvc5/build/install` exists, start the binary and verify reflection + health work locally.

```bash
cd mosaic-extensions/soter-smt
cargo run -p converge-soter-server &
sleep 2
grpcurl -plaintext localhost:50051 list
# Expect: grpc.health.v1.Health, grpc.reflection.v1.ServerReflection,
#         soter.v1.SoterSolver

grpcurl -plaintext -d '{"service": ""}' localhost:50051 grpc.health.v1.Health/Check
# Expect: {"status": "SERVING"}

grpcurl -plaintext -H 'x-converge-app: quorum-sense' \
  -d '{"query": {"query_id": "smoke", "smtlib": "(check-sat)", "timeout_ms": 5000}}' \
  localhost:50051 soter.v1.SoterSolver/Check
# Expect: SmtReport with status=SAT (empty assertions → trivially sat).

pkill -f converge-soter-server
```

This is a developer convenience step — not a milestone gate. If CVC5 isn't built locally, skip; the Cloud Build in Phase D validates end-to-end.

No commit for this task.

---

## Phase C — JSON logging + per-RPC spans

Already wired in A5 (`tracing_subscriber::fmt().json()` in main.rs, `info_span!` in service.rs). No new work.

---

## Phase D — Dockerfile + Cloud Build + Cloud Run

The GCP-touching phase. Run inline with a human watching `gcloud builds submit` (~25–35 min cold CVC5 build) and `gcloud run services replace`.

### Task D0: Prereqs (mostly shared with M1)

GCP project, AR repo, VPC connector, APIs are already set up from M1. Run a quick sanity check:

```bash
gcloud config get-value project                # expect: reflective-labs
gcloud artifacts repositories describe converge \
    --location=europe-west1 \
    --format='value(format)'                   # expect: DOCKER
gcloud compute networks vpc-access connectors describe solver-egress-ew1 \
    --region=europe-west1 \
    --format='value(state)'                    # expect: READY
```

(No commit.)

---

### Task D1: SA + IAM for `soter-server`

**Files:**
- Create: `ops/iam-setup.sh`

Mirror `mosaic-extensions/ferrox-solvers/ops/iam-setup.sh` verbatim, substituting `ferrox-server` → `soter-server` in `SA_NAME` and the display-name string. Roles granted (logging.logWriter, monitoring.metricWriter) are identical.

- [ ] **Step 1: Write `ops/iam-setup.sh`** (verbatim from ferrox-solvers with the substitution).
- [ ] **Step 2:** `chmod +x ops/iam-setup.sh && ./ops/iam-setup.sh` — creates `soter-server@reflective-labs.iam.gserviceaccount.com`.
- [ ] **Step 3: Commit:**

```bash
git add ops/iam-setup.sh
git commit -m "$(cat <<'EOF'
chore(ops): idempotent soter-server SA + IAM setup script (M2.D1)

Creates the dedicated runtime SA with logging.logWriter +
monitoring.metricWriter. No data-plane roles — solver service is pure
compute per spec §7.2.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task D2: **Self-contained `Dockerfile` (CVC5 in Stage 1)**

**This is the soter-specific dock work.** Unlike M1 (where ferrox already had a Dockerfile), soter-smt needs one from scratch. Stage 1 mirrors the `make cvc5` target from soter's `Makefile`. Stage 2 is the same shape as ferrox's Rust compile. Stage 3 is the minimal runtime.

**Files:**
- Create: `Dockerfile`

```dockerfile
# ─── Stage 1: Build CVC5 ──────────────────────────────────────────────────────
# Mirrors the `make cvc5` target. CVC5 pinned to cvc5-1.3.3 (commit
# 8ff882e3...). `--no-poly` matches Makefile's CVC5_CONFIGURE_FLAGS default.
FROM debian:trixie-slim AS cpp-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake git ca-certificates python3 python3-pip \
    libgmp-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

ARG CVC5_TAG=cvc5-1.3.3
RUN git clone --depth 1 --branch ${CVC5_TAG} \
      https://github.com/cvc5/cvc5 /build/cvc5 && \
    cd /build/cvc5 && \
    ./configure.sh production \
      --prefix=/build/cvc5/build/install \
      --auto-download \
      --no-poly && \
    cmake --build /build/cvc5/build -j"$(nproc)" && \
    cmake --install /build/cvc5/build

# ─── Stage 2: Compile Rust server ─────────────────────────────────────────────
FROM rust:1.96-trixie AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake clang libclang-dev protobuf-compiler \
    git ca-certificates pkg-config libgmp-dev \
    && rm -rf /var/lib/apt/lists/*

# Bring the CVC5 install tree into the builder. soter-cvc5-sys's build.rs
# reads SOTER_CVC5_ROOT and links against $SOTER_CVC5_ROOT/lib.
COPY --from=cpp-builder /build/cvc5/build/install /opt/cvc5

WORKDIR /workspace

# Build context is the soter-smt repo root.
COPY . /workspace/

# Clone sibling Reflective platform repos that [patch.crates-io] in
# Cargo.toml references. Mirrors the pattern from
# mosaic-extensions/ferrox-solvers/Dockerfile (M1 fixup 312605d).
RUN mkdir -p /bedrock-platform && \
    git clone --depth=1 --quiet https://github.com/Reflective-Lab/converge.git /bedrock-platform/converge

RUN sed -i \
      -e 's|path = "../../bedrock-platform/|path = "/bedrock-platform/|g' \
      /workspace/Cargo.toml && \
    rm -f /workspace/Cargo.lock

ENV SOTER_CVC5_ROOT=/opt/cvc5

# soter-cvc5-sys's build.rs `assert!(... .exists())` so a misconfigured
# SOTER_CVC5_ROOT fails the build immediately, not at runtime.
RUN test -f /opt/cvc5/include/cvc5/c/cvc5.h \
    || (echo "CVC5 headers missing at /opt/cvc5"; exit 1)

RUN cargo build --release \
    --package converge-soter-server --features converge-soter-server/full

# ─── Stage 3: Minimal runtime ─────────────────────────────────────────────────
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 libgmp10 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Ship libcvc5 + its deps alongside the binary.
COPY --from=cpp-builder /build/cvc5/build/install/lib /usr/local/lib

RUN ldconfig

COPY --from=rust-builder \
     /workspace/target/release/soter-server \
     /usr/local/bin/soter-server

VOLUME ["/tls"]
EXPOSE 50051

ENV SOTER_ADDR=0.0.0.0:50051
ENV SOTER_TLS_CERT=/tls/server.crt
ENV SOTER_TLS_KEY=/tls/server.key

ENTRYPOINT ["/usr/local/bin/soter-server"]
```

**Why this differs from ferrox's Dockerfile:**
- CVC5 build is one cmake invocation (no separate HiGHS-like Stage). `--auto-download` pulls CaDiCaL, kissat, etc. automatically — CVC5's configure script handles them.
- No abseil / protobuf gymnastics — CVC5's static-link policy is much cleaner than OR-Tools'.
- The runtime image installs `libgmp10` (CVC5 depends on libgmp at runtime).
- `--no-poly` matches the Makefile; if/when a soter caller needs LIRA/NIRA quantifier elim via libpoly, drop the flag (slower build, larger image).

**Stage 1 build time:** ~15–25 min on `E2_HIGHCPU_8` (faster than OR-Tools' ~25 min, slower than HiGHS' ~5 min).

- [ ] **Step 1: Write the Dockerfile.**

- [ ] **Step 2: Local smoke (optional, requires Docker)**

```bash
docker build -t soter-server:local-smoke -f Dockerfile .
docker run --rm -e RUST_LOG=info -p 50051:50051 soter-server:local-smoke &
sleep 3
grpcurl -plaintext localhost:50051 list
# Expect: 3 services
pkill -f "docker run"
```

(Optional. Skip if Docker isn't installed locally; Cloud Build validates the same path.)

- [ ] **Step 3: Commit:**

```bash
git add Dockerfile
git commit -m "$(cat <<'EOF'
build(docker): self-contained CVC5 + Rust Dockerfile (M2.D2)

Per spec §3.5 Rule 1, soter-smt owns its native build. Stage 1 builds
CVC5 cvc5-1.3.3 with --no-poly --auto-download (matches the Makefile's
`make cvc5` target). Stage 2 clones bedrock-platform/converge for the
patch paths (mirror of ferrox-solvers/Dockerfile post 312605d), sets
SOTER_CVC5_ROOT, builds converge-soter-server --features full. Stage 3
is a minimal Debian runtime carrying libcvc5 + libgmp10.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task D3: `docker-compose.yml` for local dev

**Files:**
- Create: `docker-compose.yml`

Mirror ferrox-solvers' compose with name + port substitutions.

```yaml
services:
  soter-server:
    build:
      context: .
      dockerfile: Dockerfile
    image: soter-server:dev
    ports:
      - "50052:50051"  # avoid clash with ferrox-server's 50051
    environment:
      - SOTER_ADDR=0.0.0.0:50051
      - RUST_LOG=soter_server=info
```

(No TLS in local dev. The 50052 host port avoids clashing with `ferrox-server` if both are run concurrently for M3 integration smoke tests.)

- [ ] **Step 1: Write the compose file.**
- [ ] **Step 2: Commit:**

```bash
git add docker-compose.yml
git commit -m "$(cat <<'EOF'
build(docker): add docker-compose for local soter-server dev (M2.D3)

Host port 50052 to avoid clash with ferrox-server on 50051 when both
extensions are running concurrently (relevant for M3 integration smoke
from a marquee-app dev environment).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task D4: Cloud Build config — mirror M1.D2

**Files:**
- Create: `ops/cloudbuild.prod.yaml`

Identical to ferrox-solvers' cloudbuild.prod.yaml except:
- Image name: `soter-server` (not `ferrox-server`)
- Timeout: keep at `3600s` (CVC5 cold build ~25 min; safe margin)

Verbatim shape (substitute as noted):

```yaml
# Cloud Build — soter-server image for Cloud Run (reflective-labs/prod).
substitutions:
  _TAG: "latest"
  _REPO: "europe-west1-docker.pkg.dev/reflective-labs/converge"

steps:
  - name: "gcr.io/cloud-builders/docker"
    env:
      - "DOCKER_BUILDKIT=1"
    args:
      - "build"
      - "-t"
      - "${_REPO}/soter-server:${_TAG}"
      - "-f"
      - "Dockerfile"
      - "."

images:
  - "${_REPO}/soter-server:${_TAG}"

options:
  machineType: "E2_HIGHCPU_8"
  logging: CLOUD_LOGGING_ONLY
  diskSizeGb: 100

timeout: "3600s"
```

- [ ] **Step 1: Write the file.**
- [ ] **Step 2: Submit a build:**

```bash
cd mosaic-extensions/soter-smt
TAG="v0.3.0-$(git rev-parse --short HEAD)"
gcloud builds submit . \
    --project=reflective-labs \
    --config=ops/cloudbuild.prod.yaml \
    --substitutions=_TAG=${TAG}
```

Expected: `SUCCESS` in ~25–35 min. The image lands in AR.

- [ ] **Step 3: Verify:**

```bash
gcloud artifacts docker images list \
    europe-west1-docker.pkg.dev/reflective-labs/converge/soter-server \
    --limit=3
```

Expected: the tag appears.

- [ ] **Step 4: Commit:**

```bash
git add ops/cloudbuild.prod.yaml
git commit -m "$(cat <<'EOF'
build(ops): Cloud Build config for soter-server prod image (M2.D4)

Targets europe-west1-docker.pkg.dev/reflective-labs/converge/soter-server.
Same machine type / timeout / disk as M1's ferrox cloudbuild. The repo's
Dockerfile is self-contained (Stage 1 CVC5, Stage 2 Rust, Stage 3 runtime).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task D5: Cloud Run manifest + deploy

**Files:**
- Create: `ops/cloudrun.prod.yaml`

Mirror ferrox-solvers' cloudrun.prod.yaml with these substitutions:
- `name: soter-server`
- `serviceAccountName: soter-server@reflective-labs.iam.gserviceaccount.com`
- `image: europe-west1-docker.pkg.dev/reflective-labs/converge/soter-server:<tag from D4>`
- env `SOTER_ADDR`, `RUST_LOG=soter_server=info`

`containerConcurrency`: keep `1` for the first deploy. CVC5 is single-process per query like CP-SAT. Revisit after load testing.

CPU / memory: **1 vCPU / 2 GiB** — SMT queries on typical reachability problems are lighter than CP-SAT. Spec §10.4 hint.

```yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: soter-server
  labels:
    app: soter-server
    extension: soter
    platform: converge
  annotations:
    run.googleapis.com/ingress: internal
spec:
  template:
    metadata:
      annotations:
        autoscaling.knative.dev/minScale: "0"   # SMT cold-start tolerable; queries aren't on the hot user path
        autoscaling.knative.dev/maxScale: "10"
        run.googleapis.com/vpc-access-connector: solver-egress-ew1
        run.googleapis.com/vpc-access-egress: private-ranges-only
    spec:
      serviceAccountName: soter-server@reflective-labs.iam.gserviceaccount.com
      containerConcurrency: 1
      timeoutSeconds: 60          # SMT queries are bounded by spec at ~30s per request
      containers:
        - image: europe-west1-docker.pkg.dev/reflective-labs/converge/soter-server:REPLACE_ME
          ports:
            - name: h2c
              containerPort: 50051
          resources:
            limits:
              cpu: "1"
              memory: "2Gi"
          env:
            - name: SOTER_ADDR
              value: "0.0.0.0:50051"
            - name: RUST_LOG
              value: "soter_server=info"
  traffic:
    - latestRevision: true
      percent: 100
```

**Why `minScale=0` for soter (vs. ferrox's `1`):** spec §10.4 says cold-start is acceptable for reachability checks since they're not on the hot user path. M3's quorum formation cycle tolerates a few seconds of warm-up. If load testing shows otherwise, bump to 1.

**Why `timeoutSeconds: 60`:** SMT queries with `produce_unsat_core` can be slower; 60s leaves headroom over the 30s spec budget.

- [ ] **Step 1: Write the file** (with `REPLACE_ME` placeholder).
- [ ] **Step 2: Substitute the tag** from D4 Step 2: `sed -i.bak 's|soter-server:REPLACE_ME|soter-server:<tag>|' ops/cloudrun.prod.yaml && rm ops/cloudrun.prod.yaml.bak`.
- [ ] **Step 3: Apply:**

```bash
gcloud run services replace ops/cloudrun.prod.yaml \
    --project=reflective-labs \
    --region=europe-west1
```

Expected: `Service [soter-server] revision [soter-server-00001-XXX] has been deployed and is serving 100 percent of traffic.`

Capture the URL.

- [ ] **Step 4: Commit:**

```bash
git add ops/cloudrun.prod.yaml
git commit -m "$(cat <<'EOF'
deploy(ops): Cloud Run service manifest for soter-server (M2.D5)

ingress=internal, VPC connector solver-egress-ew1, concurrency=1, minScale=0
(SMT cold-start acceptable per spec §10.4), maxScale=10, 1vCPU/2GiB,
timeout=60s (headroom over the spec's 30s SMT budget).
SA: soter-server@reflective-labs (no data-plane IAM).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task D6: Justfile targets — mirror M1.D4

**Files:**
- Modify: `Justfile`

Append the same four targets as ferrox-solvers' Justfile, substituting `soter` for `ferrox`:

```just
# ─── Cloud Run prod deploy (M2) ──────────────────────────────────────────────

deploy-build TAG=`echo "v0.3.0-$(git rev-parse --short HEAD)"`:
    gcloud builds submit . \
        --project=reflective-labs \
        --config=ops/cloudbuild.prod.yaml \
        --substitutions=_TAG={{TAG}}

deploy-apply:
    gcloud run services replace ops/cloudrun.prod.yaml \
        --project=reflective-labs \
        --region=europe-west1

tenants-show:
    @grep -E '^\s*Tenant \{ slug' crates/soter-server/src/tenants.rs

smoke-prod URL:
    ops/smoke.sh {{URL}}
```

- [ ] **Step 1: Append.**
- [ ] **Step 2: `just --list | grep -E 'deploy-|smoke-prod|tenants-show'` — 4 lines.**
- [ ] **Step 3: Commit:**

```bash
git add Justfile
git commit -m "$(cat <<'EOF'
chore(just): add deploy-build, deploy-apply, smoke-prod, tenants-show (M2.D6)

Wrappers mirroring ferrox-solvers' M1.D4 set, scoped to soter-server.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase E — `RemoteSmtBackend` client crate + smoke

### Task E1: Add `remote` feature to `converge-soter-smt`

**Files:**
- Modify: `crates/soter/Cargo.toml`
- Modify: `crates/soter/src/lib.rs` (later, in E2)

In `crates/soter/Cargo.toml`:

```toml
[features]
default = []
cvc5 = ["dep:soter-cvc5-sys", "soter-cvc5-sys/link"]
fake-backend = []
remote = ["dep:tonic", "dep:tonic-prost", "dep:prost", "dep:tonic-prost-build"]
```

Add to `[dependencies]`:

```toml
tonic                = { version = "0.14", features = ["tls-ring"], optional = true }
tonic-prost          = { version = "0.14", optional = true }
prost                = { version = "0.14", optional = true }
```

Add to `[build-dependencies]`:

```toml
tonic-prost-build = { version = "0.14", optional = true }
```

The `optional = true` on the build-dep is unusual — feature-gated build scripts are awkward. **Alternative:** make the build script always run but only compile protos when `cfg!(feature = "remote")`. This keeps the build-dep unconditional but cheaply skipped.

Pragmatic choice for v1: split off `remote.rs` proto codegen into a tiny helper that's only triggered by a `build.rs` block when `CARGO_FEATURE_REMOTE` env is set. Skip the optional-build-dep dance.

- [ ] **Step 1: Edit Cargo.toml** (declare deps + features per above).

- [ ] **Step 2: Add a build.rs to `crates/soter/`**

Create `crates/soter/build.rs`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("CARGO_FEATURE_REMOTE").is_some() {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let proto_dir = manifest_dir.join("../../proto");
        let proto = proto_dir.join("soter.v1.proto");
        println!("cargo:rerun-if-changed={}", proto.display());

        tonic_prost_build::configure()
            .build_server(false)
            .build_client(true)
            .compile_protos(&[proto], &[proto_dir])?;
    }
    Ok(())
}
```

Update `crates/soter/Cargo.toml`:

```toml
build = "build.rs"
```

at the top of `[package]`. And add `tonic-prost-build` to `[build-dependencies]` unconditionally (it's small):

```toml
[build-dependencies]
tonic-prost-build = "0.14"
```

- [ ] **Step 3: Commit:**

```bash
git add crates/soter/Cargo.toml crates/soter/build.rs
git commit -m "$(cat <<'EOF'
feat(soter): add `remote` feature scaffold (M2.E1)

Adds the `remote` Cargo feature + tonic/prost optional deps. build.rs
runs unconditionally but only compiles the client stubs when
CARGO_FEATURE_REMOTE is set. The next commit (M2.E2) wires
RemoteSmtBackend; this one just lays the groundwork so the diff is small.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task E2: Implement `RemoteSmtBackend`

**Files:**
- Create: `crates/soter/src/remote.rs`
- Modify: `crates/soter/src/lib.rs` — re-export `RemoteSmtBackend` behind `cfg(feature = "remote")`

```rust
// crates/soter/src/remote.rs
//! gRPC client that implements `SmtBackend` by calling a remote
//! `soter.v1.SoterSolver/Check`. Used by marquee-apps in production
//! (the `remote-smt` feature on the consuming crate).
//!
//! Per spec §5.3 and §9. Retry policy at the call site:
//! - ResourceExhausted → exp backoff, max 3
//! - DeadlineExceeded  → no retry; propagate
//! - Unavailable       → exp backoff, max 3
//! - Internal          → no retry; propagate
//!
//! Hard rule: never silently fall back to a local backend.

use std::time::Duration;

use tonic::transport::Channel;
use tonic::metadata::MetadataValue;
use tonic::Request;

use crate::backend::SmtBackend;
use crate::types::{SmtError, SmtQuery, SmtReport, SmtStatus};

// Generated by tonic-prost-build (build.rs) when `remote` is on.
pub(crate) mod proto {
    tonic::include_proto!("soter.v1");
}

use proto::soter_solver_client::SoterSolverClient;
use proto::CheckRequest as PbCheckRequest;
use proto::SmtQuery as PbSmtQuery;
use proto::SmtStatusCode;

#[derive(Clone)]
pub struct RemoteSmtBackend {
    client: SoterSolverClient<Channel>,
    tenant_app: String,
}

impl std::fmt::Debug for RemoteSmtBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteSmtBackend")
            .field("tenant_app", &self.tenant_app)
            .finish()
    }
}

impl RemoteSmtBackend {
    /// Connect to a running soter-server. `server_url` accepts:
    /// - `https://soter-server-<hash>-ew.a.run.app` (Cloud Run; tonic auto-negotiates TLS + h2)
    /// - `http://localhost:50052` (local compose; plaintext h2c)
    pub async fn new(server_url: &str, tenant_app: &str) -> Result<Self, tonic::transport::Error> {
        let endpoint = tonic::transport::Endpoint::from_shared(server_url.to_string())?
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(5));
        let channel = endpoint.connect().await?;
        Ok(Self {
            client: SoterSolverClient::new(channel),
            tenant_app: tenant_app.to_string(),
        })
    }

    fn with_metadata<T>(&self, msg: T) -> Request<T> {
        let mut req = Request::new(msg);
        let tenant_value: MetadataValue<_> = self.tenant_app.parse().expect("ascii tenant slug");
        req.metadata_mut().insert("x-converge-app", tenant_value);
        let request_id: MetadataValue<_> = uuid::Uuid::new_v4()
            .to_string()
            .parse()
            .expect("ascii uuid");
        req.metadata_mut().insert("x-request-id", request_id);
        req
    }
}

#[async_trait::async_trait]
impl SmtBackend for RemoteSmtBackend {
    fn name(&self) -> &'static str {
        "remote-cvc5"
    }

    async fn solve(&self, query: &SmtQuery) -> Result<SmtReport, SmtError> {
        let pb = PbCheckRequest {
            query: Some(PbSmtQuery {
                query_id: query.id.clone(),
                smtlib: query.smtlib.clone(),
                timeout_ms: u64::try_from(query.timeout.as_millis()).unwrap_or(u64::MAX),
                produce_model: query.produce_model,
                produce_unsat_core: query.produce_unsat_core,
            }),
        };

        let request = self.with_metadata(pb);
        let response = self
            .client
            .clone()
            .check(request)
            .await
            .map_err(|status| SmtError::Backend(format!("grpc {}: {}", status.code(), status.message())))?;

        let pb_report = response.into_inner();
        let status = match SmtStatusCode::try_from(pb_report.status).unwrap_or(SmtStatusCode::Unspecified) {
            SmtStatusCode::Sat => SmtStatus::Sat,
            SmtStatusCode::Unsat => SmtStatus::Unsat,
            SmtStatusCode::Unknown => SmtStatus::Unknown,
            SmtStatusCode::Timeout => SmtStatus::Timeout,
            SmtStatusCode::Error => SmtStatus::Error,
            SmtStatusCode::Unspecified => SmtStatus::Unknown,
        };

        Ok(SmtReport {
            status,
            solver: pb_report
                .solver_identity
                .as_ref()
                .map(|s| s.solver.clone())
                .unwrap_or_default(),
            model: pb_report.model,
            unsat_core: pb_report.unsat_core,
            diagnostics: pb_report.diagnostics,
            wall_time_seconds: pb_report.wall_time_seconds,
        })
    }
}
```

Note: This assumes `SmtError::Backend(String)` exists. Adjust to whatever variant `crates/soter/src/types.rs` actually exposes (read it first; replace `Backend(...)` with the right variant).

Update `crates/soter/src/lib.rs`:

```rust
pub mod arbiter;
pub mod backend;
pub mod cvc5;
pub mod formation;
pub mod provenance;
#[cfg(feature = "remote")]
pub mod remote;
pub mod suggestor;
pub mod types;
pub use arbiter::{/* … */};
pub use backend::{ScriptedSmtBackend, SmtBackend};
pub use cvc5::Cvc5FfiBackend;
pub use formation::{SoterCapability, formation_capabilities};
pub use provenance::{SOTER_PROVENANCE, Soter};
#[cfg(feature = "remote")]
pub use remote::RemoteSmtBackend;
pub use suggestor::SmtSuggestor;
pub use types::{SmtError, SmtEvidenceTier, SmtQuery, SmtReport, SmtStatus};
```

- [ ] **Step 1: Write `remote.rs`** (content above; adjust `SmtError` variant after reading `types.rs`).
- [ ] **Step 2: Update `lib.rs`** (gated `pub mod remote` + `pub use`).
- [ ] **Step 3: Build:** `cargo check -p converge-soter-smt --features remote` — clean.
- [ ] **Step 4: Build the workspace's other crates didn't regress:** `cargo check --workspace --no-default-features` — clean.
- [ ] **Step 5: Commit:**

```bash
git add crates/soter/src/remote.rs crates/soter/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(soter): add RemoteSmtBackend (gRPC client) behind `remote` feature (M2.E2)

RemoteSmtBackend implements SmtBackend by calling soter.v1.SoterSolver/Check
over tonic. Injects x-converge-app + x-request-id metadata on every call.
Maps SmtStatusCode <-> SmtStatus.

Per spec §5.3 / §9. Retry policy lives at the caller (marquee-app's M3
wiring will add the tower-retry middleware); this crate provides the
single-call primitive.

Hard rule: never falls back to a local backend if the remote call fails —
the caller decides degradation (skip-registration in M3's case).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task E3: Bump `converge-soter-smt` to 0.3.0

The workspace pin needs to lift the version from `0.2.3` → `0.3.0` so M3's marquee-app workspace can target the new shape via `[patch.crates-io] converge-soter-smt = { path = "..." }`.

**Files:**
- Modify: `Cargo.toml` (workspace) — bump `version.workspace` from `"0.2.3"` to `"0.3.0"`
- Modify: `CHANGELOG.md` — add a 0.3.0 entry

- [ ] **Step 1: Edit workspace Cargo.toml `version = "0.2.3"` → `"0.3.0"`** and `soter-cvc5-sys`/`soter` `version = "0.2.3"` → `"0.3.0"`.
- [ ] **Step 2: Edit CHANGELOG.md** — add:

```markdown
## 0.3.0 (2026-06-XX)

### Added
- `RemoteSmtBackend` (gRPC client) behind the `remote` Cargo feature.
- `crates/soter-server/` — new gRPC service hosting `Cvc5FfiBackend`.
- `proto/soter.v1.proto` — the wire contract (Check + Identity RPCs).

### Notes
- v1 production deploy: ingress=internal, ferrox-server@reflective-labs
  invokes `Check`. See `kb/Architecture/Cloud Run Deployment.md`.
- Publish to crates.io is a separate ticket — local path patch unblocks
  marquee-app M3 work in the meantime.
```

- [ ] **Step 3: Build + test the bumped workspace:**

```bash
cargo check --workspace --no-default-features
cargo test  --workspace --no-default-features
```

Expected: clean.

- [ ] **Step 4: Commit:**

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "$(cat <<'EOF'
chore(release): bump workspace version to 0.3.0 (M2.E3)

Adds RemoteSmtBackend + soter-server crate + proto. Publish to crates.io
deferred to a follow-up ticket. M3 marquee-app workspace will resolve via
[patch.crates-io] path patch until publish lands.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task E4: Smoke against prod

**Files:**
- Create: `ops/smoke.sh`

Mirror M1.E1 verbatim with these substitutions:
- Different service name + URL
- Different RPC payloads
- Tenant-allowlist behavior identical

```bash
#!/usr/bin/env bash
# Smoke-test the deployed soter-server.
#
# Canonical flow: run from Cloud Shell (https://shell.cloud.google.com)
#
#   1. Open Cloud Shell, project = reflective-labs
#   2. git clone --depth=1 -b next https://github.com/Reflective-Lab/soter-smt.git
#   3. cd soter-smt
#   4. ops/smoke.sh https://soter-server-640630843925.europe-west1.run.app
#
# tonic-reflection is registered server-side, so no -proto flag is needed.

set -euo pipefail

URL="${1:?usage: smoke.sh <soter-server-url>}"
HOST_PORT="${URL#https://}"
HOST_PORT="${HOST_PORT#http://}"
HOST_PORT="${HOST_PORT%/}:443"
SCHEME_FLAGS=""
[[ "$URL" == http://* ]] && SCHEME_FLAGS="-plaintext"

echo "── 1/5: Health/Check → SERVING ──"
RESP=$(grpcurl $SCHEME_FLAGS -d '{"service": ""}' "$HOST_PORT" grpc.health.v1.Health/Check)
echo "$RESP"
echo "$RESP" | grep -q '"status": "SERVING"' || { echo "FAIL"; exit 1; }
echo "ok"; echo

echo "── 2/5: list → 3 services ──"
grpcurl $SCHEME_FLAGS "$HOST_PORT" list

echo "── 3/5: Check WITHOUT x-converge-app → INVALID_ARGUMENT ──"
grpcurl $SCHEME_FLAGS -d '{"query":{"query_id":"smoke","smtlib":"(check-sat)","timeout_ms":5000}}' \
  "$HOST_PORT" soter.v1.SoterSolver/Check 2>&1 | head -4

echo "── 4/5: Check WITH unknown tenant → PERMISSION_DENIED ──"
grpcurl $SCHEME_FLAGS -H 'x-converge-app: nope-not-real' \
  -d '{"query":{"query_id":"smoke","smtlib":"(check-sat)","timeout_ms":5000}}' \
  "$HOST_PORT" soter.v1.SoterSolver/Check 2>&1 | head -4

echo "── 5/5: Check WITH quorum-sense + trivial (check-sat) ──"
grpcurl $SCHEME_FLAGS -H 'x-converge-app: quorum-sense' \
  -d '{"query":{"query_id":"smoke","smtlib":"(check-sat)","timeout_ms":5000}}' \
  "$HOST_PORT" soter.v1.SoterSolver/Check 2>&1 | head -20

echo "── ALL 5 SMOKE CHECKS PASSED ──"
```

- [ ] **Step 1: Write the file.**
- [ ] **Step 2: `chmod +x ops/smoke.sh`.**
- [ ] **Step 3: Commit:**

```bash
git add ops/smoke.sh
git commit -m "$(cat <<'EOF'
test(ops): grpcurl smoke for soter-server prod (M2.E4)

Five checks: Health/Check SERVING, list shows 3 services (Health,
ServerReflection, SoterSolver), missing tenant → INVALID_ARGUMENT,
unknown tenant → PERMISSION_DENIED, quorum-sense + trivial (check-sat) →
SmtReport. Canonical run path: Cloud Shell (inside Google's network).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4: Run the smoke from Cloud Shell**

In Cloud Shell:

```bash
git clone --depth=1 -b next https://github.com/Reflective-Lab/soter-smt.git
cd soter-smt
ops/smoke.sh https://soter-server-640630843925.europe-west1.run.app
```

Expected: all five sections print `ok`, ending with `── ALL 5 SMOKE CHECKS PASSED ──`. If any section fails, diagnose like M1.E2 (revision status, Cloud Logging for the request_id).

(No commit — this is the verification gate.)

---

## Phase F — Close-out

### Task F1: Update `kb/Architecture/Cloud Run Deployment.md`

**Files:**
- Modify: `kb/Architecture/Cloud Run Deployment.md`

Append a "Deployed state (M2 shipped)" section mirroring M1's E3 update — record the prod URL, image tag, tenant allowlist, smoke results, and any deviations.

```markdown
## Deployed state (M2 shipped YYYY-MM-DD)

| Field | Value |
|---|---|
| Project | `reflective-labs` (number 640630843925) |
| Region | `europe-west1` |
| Service URL | `https://soter-server-640630843925.europe-west1.run.app` |
| Ingress | `internal` |
| VPC connector | `solver-egress-ew1` (shared with ferrox-server) |
| Service account | `soter-server@reflective-labs.iam.gserviceaccount.com` |
| Image tag | `v0.3.0-<sha>` |
| Image registry | `europe-west1-docker.pkg.dev/reflective-labs/converge/soter-server` |
| Concurrency | 1 |
| Min / Max instances | 0 / 10 |
| CPU / Memory | 1 vCPU / 2 GiB |
| Cloud Run timeout | 60s |
| Bearer auth | off (SOTER_AUTH_TOKEN unset; tenant header is the gate) |
| Tenant allowlist | `quorum-sense` (8 in-flight) |
| Health check | `grpc.health.v1.Health` via `tonic-health` |
| Reflection | `grpc.reflection.v1.ServerReflection` via `tonic-reflection` |
| Smoke script | `ops/smoke.sh <url>` |

**Smoke verified YYYY-MM-DD:** 5/5 checks pass from Cloud Shell.

**Unblocked:** M3 (quorum-sense flips to `RemoteSmtBackend`) can now plan against this URL.
```

- [ ] **Step 1: Append the section** (with real date + image tag + smoke results).
- [ ] **Step 2: Commit + push.**

---

### Task F2: Mark M2 shipped in the spec

**Files:**
- Modify: `marquee-apps/quorum-sense/docs/superpowers/specs/2026-06-14-converge-grpc-suggestor-pattern-design.md`

Replace:

```markdown
### M2 — soter-server reaches Cloud Run prod
```

with:

```markdown
### M2 — soter-server reaches Cloud Run prod ✅ shipped YYYY-MM-DD
```

- [ ] **Step 1: Edit the line.**
- [ ] **Step 2: Commit** (single line spec update) **+ push** (on quorum-sense `next`).

---

## Self-Review checklist

Run after all phases:

- [ ] No `TODO` / `TBD` / `<fill-in>` / `REPLACE_ME` left in committed files (replace `REPLACE_ME` in cloudrun.prod.yaml is task-time substitution, expected).
- [ ] §3.5 Rule 1 satisfied: soter-smt's Dockerfile builds CVC5 in-tree, not via shared `kenneth-backend-math-base` or ferrox's Dockerfile.
- [ ] §4.2 proto matches the file (CheckRequest + SmtReport + SolverIdentity + SmtStatusCode + SoterSolver service).
- [ ] §4.3 metadata: `x-converge-app` required (interceptor.rs); `x-request-id` mint/accept (interceptor.rs); `authorization` optional (env-gated).
- [ ] §5.2 soter-server: tenant allowlist, optional bearer, `concurrency=1`, docker-compose.yml present.
- [ ] §5.3 RemoteSmtBackend: implements SmtBackend; lives in `crates/soter/src/remote.rs`; behind `remote` feature; never falls back to local backend silently.
- [ ] §5.4 feature matrix: `cvc5` ⊥ `remote` mutually exclusive in marquee-apps (verified by M3, not here; the feature flags are independent for soter itself).
- [ ] §6 tenants: compile-time `const TENANTS`, in-process semaphores, gRPC Status mapping per the spec.
- [ ] §7 network: ingress=internal, VPC connector, h2c port, ferrox-server@reflective-labs SA model carried over.
- [ ] §8 M2 exit criteria:
  - [ ] soter-server reachable inside VPC (verified via Cloud Shell smoke).
  - [ ] Health green.
  - [ ] Tenant rejection verified.
  - [ ] Local compose works (D3 verified).
  - [ ] `converge-soter-smt 0.3.0` workspace version bumped (publish deferred).
  - [ ] No marquee-app traffic yet (true; M3 wires).
- [ ] §9 observability: JSON logs with per-RPC spans (`tenant_app`, `request_id`, `rpc_method`, `solve_duration_us`, `status`, `smtlib_hash`).
- [ ] §10 out-of-scope items still out-of-scope (per-minute rate limit, bearer, prometheus dashboard, multi-region).
- [ ] kb update (F1) records prod state.
- [ ] Spec status (F2) flipped to `✅ shipped`.

---

## What unblocks after M2

- **M3 plan** can be written. Most of M3 is in quorum-sense: bump `converge-soter-smt` to 0.3.0, flip the `cvc5` cfg gates to `remote-smt`, wire `RemoteSmtBackend::new(&env::var("SOTER_SERVER_URL")?, "quorum-sense")`, drop `kenneth-backend-math-base` from Dockerfile.cloudrun, deploy. Quorum-side scope.
- **Future M4 / M5** unblocked too — the gRPC suggestor pattern now has two reference services, the second one validated the M1 template's reusability.
