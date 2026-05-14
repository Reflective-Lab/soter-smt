use std::{fmt, str::FromStr};

use converge_pack::{ContextKey, ProposalId, ProposedFact};
use tracing::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProvenanceSource {
    Soter,
}

impl ProvenanceSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Soter => "soter",
        }
    }

    pub fn proposed_fact(
        self,
        key: ContextKey,
        id: impl Into<ProposalId>,
        content: impl Into<String>,
    ) -> ProposedFact {
        ProposedFact::new(key, id, content, self.as_str())
    }
}

impl fmt::Display for ProvenanceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownProvenanceSource {
    value: String,
}

impl fmt::Display for UnknownProvenanceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown provenance source '{}'", self.value)
    }
}

impl std::error::Error for UnknownProvenanceSource {}

impl FromStr for ProvenanceSource {
    type Err = UnknownProvenanceSource;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "soter" => Ok(Self::Soter),
            other => Err(UnknownProvenanceSource {
                value: other.to_string(),
            }),
        }
    }
}

pub const SOTER_PROVENANCE: ProvenanceSource = ProvenanceSource::Soter;

pub fn suggestor_span(
    suggestor: &'static str,
    input_key: ContextKey,
    output_key: ContextKey,
    input_count: usize,
) -> Span {
    tracing::info_span!(
        "soter.suggestor.execute",
        provenance = SOTER_PROVENANCE.as_str(),
        suggestor,
        input_key = ?input_key,
        output_key = ?output_key,
        input_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_round_trips() {
        assert_eq!("soter".parse(), Ok(ProvenanceSource::Soter));
        assert_eq!(ProvenanceSource::Soter.to_string(), "soter");
    }

    #[test]
    fn proposed_fact_uses_canonical_source_string() {
        let fact = SOTER_PROVENANCE.proposed_fact(ContextKey::Evaluations, "id", "{}");
        assert_eq!(fact.provenance(), "soter");
    }
}
