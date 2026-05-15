//! Soter's `ProvenanceSource` marker.
//!
//! Migrated to the [`converge_pack::ProvenanceSource`] trait. Public
//! surface is unchanged at call sites:
//! `SOTER_PROVENANCE.proposed_fact(...)` reads the same.
//!
//! The `converge-core` engine emits a uniform `suggestor.execute`
//! tracing span automatically around every `Suggestor::execute`
//! call. Soter's Suggestors override `Suggestor::provenance()` to
//! return `SOTER_PROVENANCE.as_str()` so the engine's span carries
//! the right origin.

use converge_pack::{ContextKey, ProvenanceSource};
use tracing::Span;

/// Marker type identifying soter-emitted facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Soter;

impl ProvenanceSource for Soter {
    fn as_str(&self) -> &'static str {
        "soter"
    }
}

/// Canonical provenance constant for soter. Use it to construct
/// proposals: `SOTER_PROVENANCE.proposed_fact(key, id, payload)`.
pub const SOTER_PROVENANCE: Soter = Soter;

/// Legacy per-crate suggestor span helper.
///
/// Internal-only. The engine emits the canonical `suggestor.execute`
/// span automatically; this helper exists only as a transitional shim
/// for any in-crate call sites until they migrate to relying on the
/// engine middleware. New code does not call this.
pub(crate) fn suggestor_span(
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
    fn provenance_string_is_stable() {
        assert_eq!(SOTER_PROVENANCE.as_str(), "soter");
    }

    #[test]
    fn proposed_fact_uses_canonical_source_string() {
        let fact = SOTER_PROVENANCE.proposed_fact(
            ContextKey::Evaluations,
            "id",
            converge_pack::TextPayload::new("{}"),
        );
        assert_eq!(fact.provenance(), "soter");
    }
}
