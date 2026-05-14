use async_trait::async_trait;
use soter_cvc5_sys::{Cvc5SolveReport, Cvc5Status};

use crate::{
    backend::SmtBackend,
    types::{SmtError, SmtQuery, SmtReport, SmtStatus},
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
        self.check_sat_smt2(query)
    }
}

fn map_report(query: &SmtQuery, solver: &str, native_report: Cvc5SolveReport) -> SmtReport {
    let mut report = SmtReport::new(query, solver, map_status(native_report.status));
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

fn map_status(status: Cvc5Status) -> SmtStatus {
    match status {
        Cvc5Status::Sat => SmtStatus::Sat,
        Cvc5Status::Unsat => SmtStatus::Unsat,
        Cvc5Status::Unknown => SmtStatus::Unknown,
        Cvc5Status::Timeout => SmtStatus::Timeout,
        Cvc5Status::Error => SmtStatus::Error,
    }
}
