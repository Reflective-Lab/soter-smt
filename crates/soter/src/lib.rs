//! SMT-backed safety and policy assurance for Converge extensions.
//!
//! Soter produces searched evidence. It does not promote facts directly and it
//! does not turn SMT results into formal proof claims.

pub mod backend;
#[cfg(feature = "cvc5")]
pub mod cvc5;
pub mod formation;
pub mod provenance;
pub mod suggestor;
pub mod types;

pub use backend::{FakeSmtBackend, SmtBackend};
#[cfg(feature = "cvc5")]
pub use cvc5::Cvc5FfiBackend;
pub use formation::{SoterCapability, formation_capabilities};
pub use provenance::{ProvenanceSource, SOTER_PROVENANCE, UnknownProvenanceSource};
pub use suggestor::SmtSuggestor;
pub use types::{SmtError, SmtEvidenceTier, SmtQuery, SmtReport, SmtStatus};
