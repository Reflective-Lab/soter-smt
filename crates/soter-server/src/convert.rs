//! Proto <-> `soter::SmtQuery` / `soter::SmtReport` mapping.
//!
//! The crates split responsibility deliberately: `soter` owns the canonical
//! Rust types and their validation; this module is a pure boundary translator
//! between the wire `soter.v1` shape and those types. Every error path
//! converts into a typed `tonic::Status` so the service layer never has to
//! pattern-match on domain errors.

use sha2::{Digest, Sha256};
use soter::{SmtError, SmtQuery, SmtReport, SmtStatus};
use tonic::Status;

use crate::proto::soter::v1 as pb;

/// Translate the proto `CheckRequest` into a `soter::SmtQuery`.
///
/// Field-level validation happens here (non-empty `smtlib`, non-empty
/// `query_id`) so the backend never sees an obviously-malformed request.
/// Deeper invariants (timeout range, `(check-sat)` present, ...) are enforced
/// inside `soter::SmtQuery::validate` and surface as `SmtError::InvalidQuery`
/// inside the backend; those map to `InvalidArgument` via `smt_error_to_status`.
pub fn smt_query_from_proto(req: pb::CheckRequest) -> Result<SmtQuery, Status> {
    let Some(q) = req.query else {
        return Err(Status::invalid_argument("CheckRequest.query is required"));
    };

    if q.query_id.trim().is_empty() {
        return Err(Status::invalid_argument(
            "SmtQuery.query_id must be non-empty",
        ));
    }
    if q.smtlib.trim().is_empty() {
        return Err(Status::invalid_argument(
            "SmtQuery.smtlib must be non-empty",
        ));
    }

    Ok(SmtQuery {
        query_id: q.query_id,
        smtlib: q.smtlib,
        timeout_ms: q.timeout_ms,
        produce_model: q.produce_model,
        produce_unsat_core: q.produce_unsat_core,
    })
}

/// Translate a `soter::SmtReport` into the wire `SmtReport`.
///
/// `wall_time_seconds` is measured at the service layer (the soter report does
/// not carry wallclock today) and passed through. `query_hash` is the canonical
/// `SmtQuery::query_hash()` already present on the report.
pub fn smt_report_to_proto(report: SmtReport, wall_time_seconds: f64) -> pb::SmtReport {
    let status = match report.status {
        SmtStatus::Sat => pb::SmtStatusCode::SmtStatusSat,
        SmtStatus::Unsat => pb::SmtStatusCode::SmtStatusUnsat,
        SmtStatus::Unknown => pb::SmtStatusCode::SmtStatusUnknown,
        SmtStatus::Timeout => pb::SmtStatusCode::SmtStatusTimeout,
        SmtStatus::Error => pb::SmtStatusCode::SmtStatusError,
    };

    let native = report.execution_identity.native_identity.as_ref();
    let solver_identity = pb::SolverIdentity {
        solver: report.execution_identity.backend.clone(),
        native_backend: native
            .map(|n| n.backend.clone())
            .unwrap_or_default(),
        native_version: native
            .map(|n| n.version.clone())
            .unwrap_or_else(|| report.execution_identity.backend_version.clone()),
        rust_crate: report.execution_identity.producer.name.clone(),
        rust_crate_version: report.execution_identity.producer.version.clone(),
    };

    pb::SmtReport {
        query_id: report.query_id,
        query_hash: report.query_hash,
        status: status as i32,
        solver_identity: Some(solver_identity),
        model: report.model,
        unsat_core: report.unsat_core,
        diagnostics: report.diagnostics,
        wall_time_seconds,
    }
}

/// Map a `soter::SmtError` into a typed `tonic::Status`.
///
/// `InvalidQuery` is a client mistake (`InvalidArgument`); `Backend` and
/// `Serialize` are server-side failures (`Internal`). We never propagate the
/// raw thiserror Display into the gRPC message verbatim beyond the variant
/// prefix to keep the wire surface stable.
pub fn smt_error_to_status(err: SmtError) -> Status {
    match err {
        SmtError::InvalidQuery(msg) => Status::invalid_argument(msg),
        SmtError::Backend(msg) => Status::internal(format!("smt backend error: {msg}")),
        SmtError::Serialize(msg) => Status::internal(format!("smt serialize error: {msg}")),
    }
}

/// First 12 bytes of SHA-256 over the SMT-LIB input, hex-encoded.
///
/// Used purely for log dedup / telemetry correlation (see
/// `service::check`'s `smtlib_hash` field on the structured log lines). NOT
/// a security primitive and NOT the canonical query hash that goes into the
/// wire `SmtReport.query_hash` (that one comes from
/// `SmtQuery::query_hash()` and covers more than just the SMT-LIB text).
#[must_use]
pub fn smtlib_hash(smtlib: &str) -> String {
    let mut h = Sha256::new();
    h.update(smtlib.as_bytes());
    let digest = h.finalize();
    digest
        .iter()
        .take(12)
        .fold(String::with_capacity(24), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
}
