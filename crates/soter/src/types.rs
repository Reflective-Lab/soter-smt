use serde::{Deserialize, Serialize};
use thiserror::Error;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmtQuery {
    pub query_id: String,
    pub smtlib: String,
    pub timeout_ms: u64,
    pub produce_model: bool,
    pub produce_unsat_core: bool,
}

impl SmtQuery {
    pub fn new(query_id: impl Into<String>, smtlib: impl Into<String>) -> Self {
        Self {
            query_id: query_id.into(),
            smtlib: smtlib.into(),
            timeout_ms: 5_000,
            produce_model: true,
            produce_unsat_core: true,
        }
    }

    pub fn validate(&self) -> Result<(), SmtError> {
        if self.query_id.trim().is_empty() {
            return Err(SmtError::InvalidQuery("query_id must not be empty".into()));
        }
        if self.smtlib.trim().is_empty() {
            return Err(SmtError::InvalidQuery("smtlib must not be empty".into()));
        }
        if !self.smtlib.contains("(check-sat") {
            return Err(SmtError::InvalidQuery(
                "smtlib must include a check-sat command".into(),
            ));
        }
        if self.timeout_ms == 0 {
            return Err(SmtError::InvalidQuery("timeout_ms must be > 0".into()));
        }
        Ok(())
    }

    pub fn query_hash(&self) -> String {
        format_hash(fnv1a([
            self.query_id.as_bytes(),
            b"\0",
            self.smtlib.as_bytes(),
        ]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtStatus {
    Sat,
    Unsat,
    Unknown,
    Timeout,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtEvidenceTier {
    Searched,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmtReport {
    pub query_id: String,
    pub query_hash: String,
    pub solver: String,
    pub status: SmtStatus,
    pub evidence_tier: SmtEvidenceTier,
    pub model: Option<String>,
    pub unsat_core: Option<String>,
    pub diagnostics: Option<String>,
}

impl SmtReport {
    pub fn new(query: &SmtQuery, solver: impl Into<String>, status: SmtStatus) -> Self {
        Self {
            query_id: query.query_id.clone(),
            query_hash: query.query_hash(),
            solver: solver.into(),
            status,
            evidence_tier: SmtEvidenceTier::Searched,
            model: None,
            unsat_core: None,
            diagnostics: None,
        }
    }

    pub fn confidence(&self) -> f64 {
        match self.status {
            SmtStatus::Sat | SmtStatus::Unsat => 0.9,
            SmtStatus::Unknown => 0.2,
            SmtStatus::Timeout | SmtStatus::Error => 0.0,
        }
    }

    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    #[must_use]
    pub fn with_unsat_core(mut self, core: impl Into<String>) -> Self {
        self.unsat_core = Some(core.into());
        self
    }

    #[must_use]
    pub fn with_diagnostics(mut self, diagnostics: impl Into<String>) -> Self {
        self.diagnostics = Some(diagnostics.into());
        self
    }
}

#[derive(Debug, Error)]
pub enum SmtError {
    #[error("invalid SMT query: {0}")]
    InvalidQuery(String),
    #[error("SMT backend failed: {0}")]
    Backend(String),
    #[error("failed to serialize SMT report: {0}")]
    Serialize(String),
}

fn fnv1a<const N: usize>(parts: [&[u8]; N]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for part in parts {
        for byte in part {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

fn format_hash(hash: u64) -> String {
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_query_shape() {
        let query = SmtQuery::new("q1", "(set-logic QF_LIA)\n(check-sat)");
        assert!(query.validate().is_ok());
        assert!(query.query_hash().starts_with("fnv1a64:"));
    }

    #[test]
    fn rejects_empty_query_id() {
        let err = SmtQuery::new("", "(check-sat)").validate().unwrap_err();
        assert!(matches!(err, SmtError::InvalidQuery(_)));
    }

    #[test]
    fn searched_report_confidence_depends_on_status() {
        let query = SmtQuery::new("q1", "(check-sat)");
        assert!((SmtReport::new(&query, "fake", SmtStatus::Sat).confidence() - 0.9).abs() < 1e-9);
        assert!(
            (SmtReport::new(&query, "fake", SmtStatus::Timeout).confidence() - 0.0).abs() < 1e-9
        );
    }
}
