use async_trait::async_trait;
use converge_pack::{ExecutionIdentity, ExecutionProducerIdentity, NativeExecutionIdentity};
use soter_cvc5_sys::{Cvc5SolveReport, Cvc5Status};

use crate::{
    backend::SmtBackend,
    types::{SmtError, SmtQuery, SmtReport, SmtStatus, smt_runtime_config},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Cvc5FfiBackend;

impl Cvc5FfiBackend {
    pub fn linked_version(self) -> String {
        soter_cvc5_sys::linked_version()
    }

    pub fn check_sat_smt2(self, query: &SmtQuery) -> Result<SmtReport, SmtError> {
        query.validate()?;
        let native_report = soter_cvc5_sys::check_sat_smt2(
            &query.smtlib,
            query.timeout_ms,
            query.produce_model,
            query.produce_unsat_core,
        );
        Ok(map_report(query, self.name(), native_report))
    }
}

#[async_trait]
impl SmtBackend for Cvc5FfiBackend {
    fn name(&self) -> &'static str {
        "cvc5"
    }

    async fn solve(&self, query: &SmtQuery) -> Result<SmtReport, SmtError> {
        query.validate()?;
        let query = query.clone();
        tokio::task::spawn_blocking(move || Cvc5FfiBackend.check_sat_smt2(&query))
            .await
            .map_err(|err| SmtError::Backend(format!("cvc5 blocking worker failed: {err}")))?
    }
}

fn map_report(query: &SmtQuery, solver: &str, native_report: Cvc5SolveReport) -> SmtReport {
    let mut report = SmtReport::new_with_execution_identity(
        query,
        cvc5_execution_identity(query, solver),
        map_status(native_report.status),
    );
    if let Some(model) = native_report.model {
        report = report.with_model(model);
    }
    if let Some(unsat_core) = native_report.unsat_core {
        report = report.with_unsat_core(unsat_core);
    }
    if let Some(diagnostics) = native_report.diagnostics {
        report = report.with_diagnostics(diagnostics);
    }
    report
}

fn cvc5_execution_identity(query: &SmtQuery, solver: &str) -> ExecutionIdentity {
    let native_source_mode = soter_cvc5_sys::cvc5_source_mode();
    let build_config = if native_source_mode == "external-root" {
        format!(
            "external_root=true; reported_configure_flags={}",
            soter_cvc5_sys::cvc5_configure_flags()
        )
    } else {
        format!(
            "profile=production; auto_download=true; configure_flags={}",
            soter_cvc5_sys::cvc5_configure_flags()
        )
    };

    ExecutionIdentity::new(
        ExecutionProducerIdentity::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        solver,
        soter_cvc5_sys::linked_version(),
        build_config,
        smt_runtime_config(query),
        Some(NativeExecutionIdentity::new(
            soter_cvc5_sys::CVC5_NAME,
            soter_cvc5_sys::linked_version(),
            soter_cvc5_sys::CVC5_SOURCE_URL,
            soter_cvc5_sys::CVC5_EXPECTED_COMMIT,
            soter_cvc5_sys::cvc5_source_commit(),
            native_source_mode,
        )),
    )
}

fn map_status(status: Cvc5Status) -> SmtStatus {
    match status {
        Cvc5Status::Sat => SmtStatus::Sat,
        Cvc5Status::Unsat => SmtStatus::Unsat,
        Cvc5Status::Unknown => SmtStatus::Unknown,
        Cvc5Status::Timeout => SmtStatus::Timeout,
        Cvc5Status::Error => SmtStatus::Error,
    }
}
