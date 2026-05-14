use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoterCapability {
    pub id: &'static str,
    pub family: &'static str,
    pub surface: &'static str,
    pub evidence_tier: &'static str,
    pub description: &'static str,
}

pub fn formation_capabilities() -> Vec<SoterCapability> {
    vec![
        SoterCapability {
            id: "soter.smt.solver",
            family: "soter.smt",
            surface: "SmtSuggestor",
            evidence_tier: "searched",
            description: "Runs SMT queries and emits SmtReport evidence.",
        },
        SoterCapability {
            id: "soter.smt.cvc5_ffi",
            family: "soter.smt",
            surface: "Cvc5FfiBackend",
            evidence_tier: "searched",
            description: "Native CVC5 backend behind the explicit cvc5 feature.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_ids_are_stable() {
        let ids: Vec<_> = formation_capabilities()
            .into_iter()
            .map(|capability| capability.id)
            .collect();

        assert_eq!(ids, vec!["soter.smt.solver", "soter.smt.cvc5_ffi"]);
    }
}
