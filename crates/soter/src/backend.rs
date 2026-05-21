use async_trait::async_trait;

#[cfg(not(feature = "fake-backend"))]
use crate::types::{SmtError, SmtQuery, SmtReport};
#[cfg(feature = "fake-backend")]
use crate::types::{SmtError, SmtQuery, SmtReport, SmtStatus};

#[async_trait]
pub trait SmtBackend: Send + Sync {
    fn name(&self) -> &'static str;

    async fn solve(&self, query: &SmtQuery) -> Result<SmtReport, SmtError>;
}

/// Test fixture only. Returns scripted SMT results; never reaches a solver.
///
/// Gated behind the `fake-backend` feature so a default-features build of soter
/// cannot silently fall through to a scripted answer in place of real evidence.
/// Real SMT requires the `cvc5` feature and `Cvc5FfiBackend`.
#[cfg(feature = "fake-backend")]
#[derive(Debug, Clone)]
pub struct ScriptedSmtBackend {
    status: SmtStatus,
}

#[cfg(feature = "fake-backend")]
impl ScriptedSmtBackend {
    pub fn new(status: SmtStatus) -> Self {
        Self { status }
    }

    pub fn sat() -> Self {
        Self::new(SmtStatus::Sat)
    }

    pub fn unsat() -> Self {
        Self::new(SmtStatus::Unsat)
    }
}

#[cfg(feature = "fake-backend")]
#[async_trait]
impl SmtBackend for ScriptedSmtBackend {
    fn name(&self) -> &'static str {
        "scripted-smt"
    }

    async fn solve(&self, query: &SmtQuery) -> Result<SmtReport, SmtError> {
        query.validate()?;
        let report = match self.status {
            SmtStatus::Sat => SmtReport::new(query, self.name(), self.status).with_model("(model)"),
            SmtStatus::Unsat => {
                SmtReport::new(query, self.name(), self.status).with_unsat_core("(unsat-core)")
            }
            SmtStatus::Unknown => {
                SmtReport::new(query, self.name(), self.status).with_diagnostics("unknown")
            }
            SmtStatus::Timeout => {
                SmtReport::new(query, self.name(), self.status).with_diagnostics("timeout")
            }
            SmtStatus::Error => {
                return Err(SmtError::Backend(
                    "scripted backend configured to fail".to_string(),
                ));
            }
        };
        Ok(report)
    }
}

#[cfg(all(test, feature = "fake-backend"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scripted_backend_returns_unsat_core() {
        let query = SmtQuery::new("q1", "(check-sat)");
        let report = ScriptedSmtBackend::unsat().solve(&query).await.unwrap();

        assert_eq!(report.status, SmtStatus::Unsat);
        assert!(report.unsat_core.is_some());
    }
}
