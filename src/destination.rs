//! # Destination — What Safety Decision Cargo Moves Toward
//!
//! Every piece of PV cargo is in transit toward a safety decision. The destination
//! determines routing priority, regulatory timeline, and which processing stations
//! the cargo must pass through.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The safety decision this cargo is moving toward.
///
/// Maps to the major PV workflow endpoints. A cargo's destination can be
/// refined during transit (e.g., generic `SignalDetection` narrows to
/// `CausalityAssessment` once a signal is confirmed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Destination {
    /// Statistical signal detection (PRR, ROR, IC, EBGM)
    SignalDetection,
    /// Individual case causality assessment (Naranjo, WHO-UMC)
    CausalityAssessment,
    /// Regulatory submission (ICSR, PSUR, PBRER)
    RegulatoryReporting,
    /// Risk minimization measures (RMP, REMS)
    RiskMinimization,
    /// Benefit-risk evaluation (QBR framework)
    BenefitRiskEvaluation,
    /// Product labeling change (SmPC, USPI update)
    LabelingChange,
    /// Signal validation (confirming or refuting a detected signal)
    SignalValidation,
    /// Aggregate analysis (periodic safety update)
    AggregateAnalysis,
    /// Referral or regulatory procedure
    RegulatoryProcedure,
}

impl Destination {
    /// Whether this destination involves regulatory submission with deadlines.
    #[must_use]
    pub fn has_regulatory_deadline(&self) -> bool {
        matches!(self, Self::RegulatoryReporting | Self::RegulatoryProcedure)
    }

    /// Whether this destination is a terminal node (no further routing).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::LabelingChange | Self::RegulatoryProcedure)
    }
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SignalDetection => write!(f, "Signal Detection"),
            Self::CausalityAssessment => write!(f, "Causality Assessment"),
            Self::RegulatoryReporting => write!(f, "Regulatory Reporting"),
            Self::RiskMinimization => write!(f, "Risk Minimization"),
            Self::BenefitRiskEvaluation => write!(f, "Benefit-Risk Evaluation"),
            Self::LabelingChange => write!(f, "Labeling Change"),
            Self::SignalValidation => write!(f, "Signal Validation"),
            Self::AggregateAnalysis => write!(f, "Aggregate Analysis"),
            Self::RegulatoryProcedure => write!(f, "Regulatory Procedure"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regulatory_deadline_destinations() {
        assert!(Destination::RegulatoryReporting.has_regulatory_deadline());
        assert!(Destination::RegulatoryProcedure.has_regulatory_deadline());
        assert!(!Destination::SignalDetection.has_regulatory_deadline());
        assert!(!Destination::CausalityAssessment.has_regulatory_deadline());
    }

    #[test]
    fn test_terminal_destinations() {
        assert!(Destination::LabelingChange.is_terminal());
        assert!(Destination::RegulatoryProcedure.is_terminal());
        assert!(!Destination::SignalDetection.is_terminal());
    }

    #[test]
    fn test_display() {
        assert_eq!(Destination::SignalDetection.to_string(), "Signal Detection");
        assert_eq!(
            Destination::BenefitRiskEvaluation.to_string(),
            "Benefit-Risk Evaluation"
        );
    }
}
