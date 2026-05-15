use converge_pack::{ExecutionIdentity, FactPayload};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const DEFAULT_TIMEOUT_MS: u64 = 5_000;
pub const MAX_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmtQuery {
    pub query_id: String,
    pub smtlib: String,
    pub timeout_ms: u64,
    pub produce_model: bool,
    pub produce_unsat_core: bool,
}

impl FactPayload for SmtQuery {
    const FAMILY: &'static str = "soter.smt.query";
    const VERSION: u16 = 1;
}

impl SmtQuery {
    pub fn new(query_id: impl Into<String>, smtlib: impl Into<String>) -> Self {
        Self {
            query_id: query_id.into(),
            smtlib: smtlib.into(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
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
        if self.timeout_ms > MAX_TIMEOUT_MS {
            return Err(SmtError::InvalidQuery(format!(
                "timeout_ms must be <= {MAX_TIMEOUT_MS}"
            )));
        }
        Ok(())
    }

    pub fn query_hash(&self) -> String {
        let timeout_ms = self.timeout_ms.to_string();
        let produce_model = self.produce_model.to_string();
        let produce_unsat_core = self.produce_unsat_core.to_string();
        let mut hasher = Sha256::new();
        update_hash_part(&mut hasher, self.query_id.as_bytes());
        update_hash_part(&mut hasher, self.smtlib.as_bytes());
        update_hash_part(&mut hasher, timeout_ms.as_bytes());
        update_hash_part(&mut hasher, produce_model.as_bytes());
        update_hash_part(&mut hasher, produce_unsat_core.as_bytes());
        format_hash(hasher.finalize().as_slice())
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
#[serde(deny_unknown_fields)]
pub struct SmtReport {
    pub query_id: String,
    pub query_hash: String,
    pub solver: String,
    pub execution_identity: ExecutionIdentity,
    pub status: SmtStatus,
    pub evidence_tier: SmtEvidenceTier,
    pub model: Option<String>,
    pub unsat_core: Option<String>,
    pub diagnostics: Option<String>,
}

impl FactPayload for SmtReport {
    const FAMILY: &'static str = "soter.smt.report";
    const VERSION: u16 = 1;
}

impl SmtReport {
    pub fn new(query: &SmtQuery, solver: impl Into<String>, status: SmtStatus) -> Self {
        let solver = solver.into();
        let execution_identity = ExecutionIdentity::non_native(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            solver.clone(),
            smt_runtime_config(query),
        );
        Self::new_with_execution_identity(query, execution_identity, status)
    }

    pub fn new_with_execution_identity(
        query: &SmtQuery,
        execution_identity: ExecutionIdentity,
        status: SmtStatus,
    ) -> Self {
        Self {
            query_id: query.query_id.clone(),
            query_hash: query.query_hash(),
            solver: execution_identity.backend.clone(),
            execution_identity,
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

/// Typed runtime-config view of an `SmtQuery`. JSON-serialized into
/// `ExecutionIdentity.runtime_config` per the workspace
/// `Runtime Config Encoding` standard.
#[derive(serde::Serialize)]
struct SmtRuntimeConfig {
    timeout_ms: u64,
    produce_model: bool,
    produce_unsat_core: bool,
}

pub fn smt_runtime_config(query: &SmtQuery) -> String {
    converge_pack::ExecutionIdentity::runtime_config_from_typed(&SmtRuntimeConfig {
        timeout_ms: query.timeout_ms,
        produce_model: query.produce_model,
        produce_unsat_core: query.produce_unsat_core,
    })
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

fn update_hash_part(hasher: &mut Sha256, part: &[u8]) {
    hasher.update((part.len() as u64).to_le_bytes());
    hasher.update(part);
}

fn format_hash(hash: &[u8]) -> String {
    let mut out = String::from("sha256:");
    for byte in hash {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_query_shape() {
        let query = SmtQuery::new("q1", "(set-logic QF_LIA)\n(check-sat)");
        assert!(query.validate().is_ok());
        assert!(query.query_hash().starts_with("sha256:"));
    }

    #[test]
    fn rejects_empty_query_id() {
        let err = SmtQuery::new("", "(check-sat)").validate().unwrap_err();
        assert!(matches!(err, SmtError::InvalidQuery(_)));
    }

    #[test]
    fn rejects_timeout_above_operational_cap() {
        let mut query = SmtQuery::new("q1", "(check-sat)");
        query.timeout_ms = MAX_TIMEOUT_MS + 1;

        let err = query.validate().unwrap_err();
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

    #[test]
    fn report_records_execution_identity() {
        let query = SmtQuery::new("q1", "(check-sat)");
        let report = SmtReport::new(&query, "fake", SmtStatus::Sat);

        assert_eq!(report.execution_identity.backend, "fake");
        assert!(report.execution_identity.native_identity.is_none());
        // runtime_config is the JSON encoding of the typed
        // `SmtRuntimeConfig` (workspace `Runtime Config Encoding`
        // standard); verify the JSON shape carries the timeout key.
        let parsed: serde_json::Value =
            serde_json::from_str(&report.execution_identity.runtime_config)
                .expect("runtime_config must be valid JSON");
        assert!(
            parsed.get("timeout_ms").is_some(),
            "runtime_config JSON should carry timeout_ms; got: {}",
            report.execution_identity.runtime_config
        );
    }

    #[test]
    fn execution_identity_rejects_unknown_fields() {
        let identity =
            ExecutionIdentity::non_native("converge-soter-smt", "0.1.0", "fake", "timeout_ms=1");
        let mut value = serde_json::to_value(identity).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".to_string(), serde_json::json!("field"));

        let err = serde_json::from_value::<ExecutionIdentity>(value).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn report_requires_execution_identity() {
        let report = serde_json::json!({
            "query_id": "q1",
            "query_hash": "sha256:abc",
            "solver": "fake",
            "status": "sat",
            "evidence_tier": "searched",
            "model": null,
            "unsat_core": null,
            "diagnostics": null
        });

        let err = serde_json::from_value::<SmtReport>(report).unwrap_err();
        assert!(
            err.to_string()
                .contains("missing field `execution_identity`")
        );
    }

    #[test]
    fn query_hash_includes_execution_options() {
        let base = SmtQuery::new("q1", "(check-sat)");
        let mut changed_timeout = base.clone();
        changed_timeout.timeout_ms += 1;
        let mut changed_model = base.clone();
        changed_model.produce_model = false;
        let mut changed_core = base.clone();
        changed_core.produce_unsat_core = false;

        assert_ne!(base.query_hash(), changed_timeout.query_hash());
        assert_ne!(base.query_hash(), changed_model.query_hash());
        assert_ne!(base.query_hash(), changed_core.query_hash());
    }
}
