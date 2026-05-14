use async_trait::async_trait;

use crate::types::{SmtError, SmtQuery, SmtReport, SmtStatus};

#[async_trait]
pub trait SmtBackend: Send + Sync {
    fn name(&self) -> &'static str;

    async fn solve(&self, query: &SmtQuery) -> Result<SmtReport, SmtError>;
}

#[derive(Debug, Clone)]
pub struct FakeSmtBackend {
    status: SmtStatus,
}

impl FakeSmtBackend {
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

#[async_trait]
impl SmtBackend for FakeSmtBackend {
    fn name(&self) -> &'static str {
        "fake-smt"
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
                    "fake backend configured to fail".to_string(),
                ));
            }
        };
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_backend_returns_unsat_core() {
        let query = SmtQuery::new("q1", "(check-sat)");
        let report = FakeSmtBackend::unsat().solve(&query).await.unwrap();

        assert_eq!(report.status, SmtStatus::Unsat);
        assert!(report.unsat_core.is_some());
    }
}
