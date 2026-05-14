use converge_pack::{
    ContentHash, Context, ContextFact, ContextKey, FactActor, FactActorKind, FactPromotionRecord,
    FactRemoteTrace, FactTraceLink, FactValidationSummary, Suggestor, Timestamp,
};
use soter::{FakeSmtBackend, SmtQuery, SmtStatus, SmtSuggestor};
use std::collections::HashMap;

struct MockContext {
    facts: HashMap<ContextKey, Vec<ContextFact>>,
}

impl MockContext {
    fn with_seed(content: String) -> Self {
        let mut facts = HashMap::new();
        facts.insert(
            ContextKey::Seeds,
            vec![ContextFact::new_projection(
                ContextKey::Seeds,
                "seed-1",
                content,
                FactPromotionRecord::new_projection(
                    "test-gate",
                    ContentHash::zero(),
                    FactActor::new_projection("test", FactActorKind::System),
                    FactValidationSummary::new_projection(vec![], vec![], vec![]),
                    vec![],
                    FactTraceLink::Remote(FactRemoteTrace::new_projection(
                        "test",
                        "trace:test",
                        None,
                        None,
                    )),
                    Timestamp::epoch(),
                ),
                Timestamp::epoch(),
            )],
        );
        Self { facts }
    }
}

impl Context for MockContext {
    fn has(&self, key: ContextKey) -> bool {
        self.facts.get(&key).is_some_and(|facts| !facts.is_empty())
    }

    fn get(&self, key: ContextKey) -> &[ContextFact] {
        self.facts.get(&key).map_or(&[], Vec::as_slice)
    }
}

#[tokio::test]
async fn smt_suggestor_emits_searched_report() {
    let query = SmtQuery::new("arbiter.expense.non_finance_commit", "(check-sat)");
    let ctx = MockContext::with_seed(serde_json::to_string(&query).unwrap());
    let suggestor = SmtSuggestor::new(FakeSmtBackend::unsat());

    assert!(suggestor.accepts(&ctx));
    let effect = suggestor.execute(&ctx).await;

    assert_eq!(effect.proposals().len(), 1);
    assert_eq!(effect.proposals()[0].key, ContextKey::Evaluations);
    assert_eq!(effect.proposals()[0].provenance(), "soter");
    assert!(effect.proposals()[0].content().contains("\"searched\""));
    assert!(effect.proposals()[0].content().contains("\"unsat\""));
}

#[tokio::test]
async fn smt_suggestor_routes_backend_errors_to_diagnostics() {
    let query = SmtQuery::new("bad", "(check-sat)");
    let ctx = MockContext::with_seed(serde_json::to_string(&query).unwrap());
    let suggestor = SmtSuggestor::new(FakeSmtBackend::new(SmtStatus::Error));

    let effect = suggestor.execute(&ctx).await;

    assert_eq!(effect.proposals().len(), 1);
    assert_eq!(effect.proposals()[0].key, ContextKey::Diagnostic);
}
