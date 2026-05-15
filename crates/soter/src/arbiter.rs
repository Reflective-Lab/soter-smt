use serde::{Deserialize, Serialize};

use crate::types::{SmtError, SmtQuery};

pub const EXPENSE_NON_FINANCE_HIGH_VALUE_COMMIT_INVARIANT_ID: &str =
    "arbiter.expense.non_finance_commit.high_value";

/// Abstract policy model used for the first Arbiter/Soter counterexample
/// fixture. This is not a Cedar semantics model; it is a deterministic SMT
/// model for one high-risk invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArbiterExpensePolicyModel {
    /// High-value expense commits imply the actor belongs to finance.
    StrictFinanceOnlyHighValueCommit,
    /// Intentionally vulnerable model used by negative tests.
    BrokenApprovedSupervisoryCommit,
}

impl ArbiterExpensePolicyModel {
    pub const fn stable_label(self) -> &'static str {
        match self {
            Self::StrictFinanceOnlyHighValueCommit => "strict_finance_only_high_value_commit",
            Self::BrokenApprovedSupervisoryCommit => "broken_approved_supervisory_commit",
        }
    }
}

/// Typed input for the first Arbiter invariant that Soter can render to
/// SMT-LIB and execute through CVC5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArbiterExpenseCommitInvariant {
    pub invariant_id: String,
    pub policy_model: ArbiterExpensePolicyModel,
    pub high_value_threshold_eur: i64,
    pub timeout_ms: u64,
}

impl ArbiterExpenseCommitInvariant {
    pub fn strict() -> Self {
        Self {
            invariant_id: EXPENSE_NON_FINANCE_HIGH_VALUE_COMMIT_INVARIANT_ID.to_string(),
            policy_model: ArbiterExpensePolicyModel::StrictFinanceOnlyHighValueCommit,
            high_value_threshold_eur: 5_000,
            timeout_ms: 5_000,
        }
    }

    pub fn broken() -> Self {
        Self {
            policy_model: ArbiterExpensePolicyModel::BrokenApprovedSupervisoryCommit,
            ..Self::strict()
        }
    }

    #[must_use]
    pub fn with_threshold(mut self, threshold_eur: i64) -> Self {
        self.high_value_threshold_eur = threshold_eur;
        self
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn to_smt_query(&self) -> Result<SmtQuery, SmtError> {
        self.validate()?;

        let mut query = SmtQuery::new(self.query_id(), self.render_smtlib());
        query.timeout_ms = self.timeout_ms;
        query.produce_model = true;
        query.produce_unsat_core = true;
        query.validate()?;
        Ok(query)
    }

    fn validate(&self) -> Result<(), SmtError> {
        if self.invariant_id.trim().is_empty() {
            return Err(SmtError::InvalidQuery(
                "arbiter invariant id must not be empty".to_string(),
            ));
        }
        if self.high_value_threshold_eur <= 0 {
            return Err(SmtError::InvalidQuery(
                "high_value_threshold_eur must be > 0".to_string(),
            ));
        }
        if self.timeout_ms == 0 {
            return Err(SmtError::InvalidQuery("timeout_ms must be > 0".to_string()));
        }
        Ok(())
    }

    fn query_id(&self) -> String {
        format!("{}.{}", self.invariant_id, self.policy_model.stable_label())
    }

    fn render_smtlib(&self) -> String {
        let policy_assertion = match self.policy_model {
            ArbiterExpensePolicyModel::StrictFinanceOnlyHighValueCommit => format!(
                r"(assert (!
  (=> (and action_commit resource_expense (> amount {threshold}) allow)
      principal_finance)
  :named policy_high_value_commit_requires_finance))",
                threshold = self.high_value_threshold_eur
            ),
            ArbiterExpensePolicyModel::BrokenApprovedSupervisoryCommit => format!(
                r"(assert (!
  (=> (and action_commit
           resource_expense
           principal_supervisory
           (> amount {threshold})
           receipt_gate_passed
           manager_approval_gate_passed
           required_gates_met
           human_approval_present)
      allow)
  :named broken_policy_allows_approved_supervisory_commit))",
                threshold = self.high_value_threshold_eur
            ),
        };

        format!(
            r"; generated-by: soter.arbiter
; invariant: {invariant_id}
; model: {model}
; meaning: sat = counterexample found, unsat = no counterexample in this abstraction
(set-logic QF_LIA)
(set-option :produce-models true)
(set-option :produce-unsat-cores true)

(declare-const principal_finance Bool)
(declare-const principal_supervisory Bool)
(declare-const action_commit Bool)
(declare-const resource_expense Bool)
(declare-const amount Int)
(declare-const receipt_gate_passed Bool)
(declare-const manager_approval_gate_passed Bool)
(declare-const required_gates_met Bool)
(declare-const human_approval_present Bool)
(declare-const allow Bool)

{policy_assertion}

(assert (! (not principal_finance) :named claim_principal_non_finance))
(assert (! principal_supervisory :named claim_principal_supervisory))
(assert (! action_commit :named claim_action_commit))
(assert (! resource_expense :named claim_resource_expense))
(assert (! (> amount {threshold}) :named claim_amount_above_threshold))
(assert (! receipt_gate_passed :named claim_receipt_gate_passed))
(assert (! manager_approval_gate_passed :named claim_manager_approval_gate_passed))
(assert (! required_gates_met :named claim_required_gates_met))
(assert (! human_approval_present :named claim_human_approval_present))
(assert (! allow :named claim_policy_allows_request))

(check-sat)
",
            invariant_id = self.invariant_id,
            model = self.policy_model.stable_label(),
            threshold = self.high_value_threshold_eur,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_strict_query_deterministically() {
        let left = ArbiterExpenseCommitInvariant::strict()
            .to_smt_query()
            .unwrap();
        let right = ArbiterExpenseCommitInvariant::strict()
            .to_smt_query()
            .unwrap();

        assert_eq!(left.smtlib, right.smtlib);
        assert_eq!(left.query_hash(), right.query_hash());
        assert!(
            left.smtlib
                .contains("policy_high_value_commit_requires_finance")
        );
    }

    #[test]
    fn threshold_changes_query_hash() {
        let standard = ArbiterExpenseCommitInvariant::strict()
            .to_smt_query()
            .unwrap();
        let larger = ArbiterExpenseCommitInvariant::strict()
            .with_threshold(10_000)
            .to_smt_query()
            .unwrap();

        assert_ne!(standard.smtlib, larger.smtlib);
        assert_ne!(standard.query_hash(), larger.query_hash());
    }

    #[test]
    fn rejects_invalid_typed_query_values() {
        let err = ArbiterExpenseCommitInvariant::strict()
            .with_threshold(0)
            .to_smt_query()
            .unwrap_err();
        assert!(matches!(err, SmtError::InvalidQuery(_)));

        let err = ArbiterExpenseCommitInvariant::strict()
            .with_timeout(0)
            .to_smt_query()
            .unwrap_err();
        assert!(matches!(err, SmtError::InvalidQuery(_)));
    }
}
