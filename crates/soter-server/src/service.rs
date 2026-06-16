//! gRPC service implementation for `soter.v1.SoterSolver`.
//!
//! Wraps an `Arc<dyn SmtBackend>` (production: `Cvc5FfiBackend`). Per-tenant
//! in-flight permits are taken from the `TenantRegistry` before invoking the
//! backend, so a noisy tenant cannot crowd out the others. Every RPC emits a
//! structured tracing span carrying the validated tenant slug and request-id
//! attached by the interceptor.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use converge_soter_server::interceptor::RequestId;
use converge_soter_server::tenants::{TenantRegistry, TenantSlug};
use soter::SmtBackend;

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

#[allow(clippy::result_large_err)]
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

        // Per-tenant permit first. If this tenant is over its in-flight cap we
        // fail fast (RESOURCE_EXHAUSTED) before touching the backend.
        let _tenant_permit = self.tenants.acquire(tenant_slug).await?;

        let inner = request.into_inner();
        let query = smt_query_from_proto(inner)?;
        let smtlib_h = smtlib_hash(&query.smtlib);

        let result = self
            .backend
            .solve(&query)
            .await
            .map_err(smt_error_to_status);
        let elapsed = started.elapsed();
        let elapsed_us = elapsed.as_micros();
        let wall_time_seconds = elapsed.as_secs_f64();

        match result {
            Ok(report) => {
                span.in_scope(|| {
                    tracing::info!(
                        solve_duration_us = elapsed_us,
                        status = "ok",
                        smtlib_hash = %smtlib_h,
                        "check completed"
                    );
                });
                Ok(Response::new(smt_report_to_proto(report, wall_time_seconds)))
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
        // Identity is still inside the with_interceptor stack in main.rs, so
        // the `x-converge-app` header is required for it too. That is fine for
        // v1 (one caller, always sends the header). If we ever want
        // unauthenticated capability probing, split this onto a separate
        // `.add_service()` registration without the interceptor.
        Ok(Response::new(SolverIdentity {
            solver: self.backend.name().to_string(),
            native_backend: native_backend_name().to_string(),
            native_version: native_version(),
            rust_crate: "converge-soter-smt".to_string(),
            rust_crate_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}

#[cfg(feature = "full")]
fn native_backend_name() -> &'static str {
    soter_cvc5_sys::CVC5_NAME
}

#[cfg(not(feature = "full"))]
fn native_backend_name() -> &'static str {
    "unknown"
}

#[cfg(feature = "full")]
fn native_version() -> String {
    soter_cvc5_sys::linked_version()
}

#[cfg(not(feature = "full"))]
fn native_version() -> String {
    "unknown".to_string()
}
