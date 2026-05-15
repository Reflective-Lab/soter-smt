use proptest::prelude::*;
use soter::{ArbiterExpenseCommitInvariant, SmtError};

#[cfg(feature = "cvc5")]
use soter::{Cvc5FfiBackend, SmtBackend, SmtStatus};

#[test]
fn strict_and_broken_models_render_distinct_queries() {
    let strict = ArbiterExpenseCommitInvariant::strict()
        .to_smt_query()
        .unwrap();
    let broken = ArbiterExpenseCommitInvariant::broken()
        .to_smt_query()
        .unwrap();

    assert_ne!(strict.smtlib, broken.smtlib);
    assert_ne!(strict.query_hash(), broken.query_hash());
    assert!(strict.smtlib.contains("sat = counterexample found"));
    assert!(
        broken
            .smtlib
            .contains("broken_policy_allows_approved_supervisory_commit")
    );
}

#[cfg(feature = "cvc5")]
#[tokio::test]
async fn strict_expense_model_has_no_non_finance_high_value_counterexample() {
    let query = ArbiterExpenseCommitInvariant::strict()
        .to_smt_query()
        .unwrap();
    let report = Cvc5FfiBackend.solve(&query).await.unwrap();

    assert_eq!(report.status, SmtStatus::Unsat);
    assert!(
        report
            .unsat_core
            .as_deref()
            .is_some_and(|core| core.contains("unsat-core") && core.contains("principal_finance"))
    );
}

#[cfg(feature = "cvc5")]
#[tokio::test]
async fn broken_expense_model_finds_non_finance_high_value_counterexample() {
    let query = ArbiterExpenseCommitInvariant::broken()
        .to_smt_query()
        .unwrap();
    let report = Cvc5FfiBackend.solve(&query).await.unwrap();

    assert_eq!(report.status, SmtStatus::Sat);
    assert!(
        report
            .model
            .as_deref()
            .is_some_and(|model| model.contains("allow") && model.contains("principal_finance"))
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn generated_queries_are_stable_for_valid_thresholds(threshold in 1_i64..100_000) {
        let left = ArbiterExpenseCommitInvariant::strict()
            .with_threshold(threshold)
            .to_smt_query()
            .unwrap();
        let right = ArbiterExpenseCommitInvariant::strict()
            .with_threshold(threshold)
            .to_smt_query()
            .unwrap();

        let needle = format!("> amount {threshold}");

        prop_assert_eq!(&left.smtlib, &right.smtlib);
        prop_assert_eq!(left.query_hash(), right.query_hash());
        prop_assert!(left.smtlib.contains(&needle));
    }

    #[test]
    fn generated_queries_reject_non_positive_thresholds(threshold in -10_000_i64..=0) {
        let result = ArbiterExpenseCommitInvariant::strict()
            .with_threshold(threshold)
            .to_smt_query();

        prop_assert!(matches!(result, Err(SmtError::InvalidQuery(_))));
    }
}
