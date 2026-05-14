#![cfg(feature = "cvc5")]

use soter::{Cvc5FfiBackend, SmtBackend, SmtQuery, SmtStatus};

#[test]
fn linked_cvc5_reports_version() {
    let backend = Cvc5FfiBackend;
    let version = backend.linked_version();

    assert!(!version.trim().is_empty());
    assert!(
        version.to_ascii_lowercase().contains("cvc5")
            || version.chars().any(|ch| ch.is_ascii_digit()),
        "unexpected CVC5 version string: {version}"
    );
}

#[tokio::test]
async fn cvc5_solves_sat_smtlib() {
    let backend = Cvc5FfiBackend;
    let query = SmtQuery::new(
        "cvc5.sat",
        r"
(set-logic QF_LIA)
(declare-fun x () Int)
(assert (> x 3))
(check-sat)
",
    );

    let report = backend.solve(&query).await.unwrap();

    assert_eq!(report.solver, "cvc5");
    assert_eq!(report.status, SmtStatus::Sat);
    assert!(
        report
            .model
            .as_deref()
            .is_some_and(|model| model.contains('x'))
    );
}

#[tokio::test]
async fn cvc5_solves_unsat_smtlib() {
    let backend = Cvc5FfiBackend;
    let query = SmtQuery::new(
        "cvc5.unsat",
        r"
(set-logic QF_LIA)
(declare-fun x () Int)
(assert (! (> x 3) :named gt3))
(assert (! (< x 0) :named lt0))
(check-sat)
",
    );

    let report = backend.solve(&query).await.unwrap();

    assert_eq!(report.status, SmtStatus::Unsat);
    assert!(
        report
            .unsat_core
            .as_deref()
            .is_some_and(|core| core.contains("unsat-core"))
    );
}

#[tokio::test]
async fn cvc5_reports_invalid_smtlib_as_error_status() {
    let backend = Cvc5FfiBackend;
    let query = SmtQuery::new(
        "cvc5.invalid",
        r"
(set-logic QF_LIA)
(assert (> undeclared 0))
(check-sat)
",
    );

    let report = backend.solve(&query).await.unwrap();

    assert_eq!(report.status, SmtStatus::Error);
    assert!(report.diagnostics.is_some());
}
