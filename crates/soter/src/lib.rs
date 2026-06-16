//! SMT-backed safety and policy assurance for Converge extensions.
//!
//! Soter produces searched evidence. It does not promote facts directly and it
//! does not turn SMT results into formal proof claims.

pub mod arbiter;
pub mod backend;
#[cfg(feature = "cvc5")]
pub mod cvc5;
pub mod formation;
pub mod provenance;
#[cfg(feature = "remote")]
pub mod remote;
pub mod suggestor;
pub mod types;

pub use arbiter::{
    ArbiterExpenseCommitInvariant, ArbiterExpensePolicyModel,
    EXPENSE_NON_FINANCE_HIGH_VALUE_COMMIT_INVARIANT_ID,
};
#[cfg(feature = "fake-backend")]
pub use backend::ScriptedSmtBackend;
pub use backend::SmtBackend;
#[cfg(feature = "cvc5")]
pub use cvc5::Cvc5FfiBackend;
pub use formation::{SoterCapability, formation_capabilities};
pub use provenance::{SOTER_PROVENANCE, Soter};
#[cfg(feature = "remote")]
pub use remote::RemoteSmtBackend;
pub use suggestor::SmtSuggestor;
pub use types::{SmtError, SmtEvidenceTier, SmtQuery, SmtReport, SmtStatus};
