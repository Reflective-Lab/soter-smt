use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, Suggestor};
use tracing::Instrument;

use crate::backend::SmtBackend;
use crate::provenance::{SOTER_PROVENANCE, suggestor_span};
use crate::types::{SmtError, SmtQuery};

pub struct SmtSuggestor<B> {
    backend: B,
    input_key: ContextKey,
    output_key: ContextKey,
}

impl<B> SmtSuggestor<B>
where
    B: SmtBackend,
{
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            input_key: ContextKey::Seeds,
            output_key: ContextKey::Evaluations,
        }
    }

    #[must_use]
    pub fn with_keys(mut self, input_key: ContextKey, output_key: ContextKey) -> Self {
        self.input_key = input_key;
        self.output_key = output_key;
        self
    }
}

#[async_trait]
impl<B> Suggestor for SmtSuggestor<B>
where
    B: SmtBackend + 'static,
{
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "smt-solver"
    }

    fn dependencies(&self) -> &[ContextKey] {
        std::slice::from_ref(&self.input_key)
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(self.input_key) && !ctx.has(self.output_key)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let span = suggestor_span(
            "smt-solver",
            self.input_key,
            self.output_key,
            ctx.count(self.input_key),
        );

        async move {
            let mut proposals = Vec::new();
            for fact in ctx.get(self.input_key) {
                let query = match serde_json::from_str::<SmtQuery>(fact.content()) {
                    Ok(query) => query,
                    Err(err) => {
                        proposals.push(diagnostic(
                            format!("smt-parse-error-{}", fact.id()),
                            SmtError::InvalidQuery(err.to_string()).to_string(),
                        ));
                        continue;
                    }
                };

                match self.backend.solve(&query).await {
                    Ok(report) => match serde_json::to_string(&report) {
                        Ok(content) => proposals.push(
                            SOTER_PROVENANCE
                                .proposed_fact(
                                    self.output_key,
                                    format!("smt-report-{}", report.query_id),
                                    content,
                                )
                                .with_confidence(report.confidence()),
                        ),
                        Err(err) => proposals.push(diagnostic(
                            format!("smt-serialize-error-{}", query.query_id),
                            SmtError::Serialize(err.to_string()).to_string(),
                        )),
                    },
                    Err(err) => proposals.push(diagnostic(
                        format!("smt-backend-error-{}", query.query_id),
                        err.to_string(),
                    )),
                }
            }

            AgentEffect::with_proposals(proposals)
        }
        .instrument(span)
        .await
    }
}

fn diagnostic(id: impl Into<String>, message: impl Into<String>) -> converge_pack::ProposedFact {
    SOTER_PROVENANCE.proposed_fact(
        ContextKey::Diagnostic,
        id.into(),
        serde_json::json!({
            "source": "soter",
            "message": message.into(),
        })
        .to_string(),
    )
}
