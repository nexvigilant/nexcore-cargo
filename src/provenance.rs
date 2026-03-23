//! # Provenance — Where Cargo Originated
//!
//! Every piece of cargo has a loading dock: the data source, query parameters,
//! timestamp, and initial data quality confidence. Provenance is immutable once
//! set — you cannot change where something came from.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Known data sources in the PV ecosystem.
///
/// Each variant maps to a NexVigilant Station config or external API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataSource {
    /// FDA Adverse Event Reporting System (openFDA API)
    Faers,
    /// European PV database
    EudraVigilance,
    /// WHO global ICSR database
    VigiAccess,
    /// Published medical literature (PubMed)
    Literature,
    /// FDA drug labeling (DailyMed)
    DailyMed,
    /// Clinical trial safety data (ClinicalTrials.gov)
    ClinicalTrials,
    /// WHO-UMC VigiBase
    VigiBase,
    /// European Medicines Agency regulatory data
    Ema,
    /// OpenVigil France disproportionality service
    OpenVigil,
    /// Drug interaction/pharmacology database
    DrugBank,
    /// RxNorm drug nomenclature (RxNav)
    RxNav,
    /// MedDRA medical terminology
    MedDra,
    /// ICH regulatory guidelines
    Ich,
    /// FDA safety communications / MedWatch
    FdaSafety,
    /// FDA drug approvals and labeling changes
    FdaAccessdata,
    /// Internal NexCore computation (signal pipeline, causality engine)
    Internal(String),
    /// Custom / unlisted source
    Other(String),
}

/// Query parameters that produced this cargo.
///
/// Preserves the exact request so cargo can be traced back to its origin query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryParams {
    /// Key-value pairs of the query (e.g., drug="metformin", event="lactic acidosis")
    pub params: BTreeMap<String, String>,
}

impl QueryParams {
    /// Create empty query params.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            params: BTreeMap::new(),
        }
    }

    /// Create query params from key-value pairs.
    #[must_use]
    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            params: pairs.into_iter().collect(),
        }
    }

    /// Add a parameter.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.insert(key.into(), value.into());
    }
}

/// Where cargo originated — the loading dock.
///
/// Provenance is set at cargo creation and never modified. It answers:
/// "Where did this data come from, what query produced it, and how
/// confident are we in the source?"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// The data source (FAERS, EudraVigilance, etc.)
    pub source: DataSource,
    /// The query that produced this cargo
    pub query: QueryParams,
    /// When this cargo was loaded (Unix timestamp seconds)
    pub loaded_at: i64,
    /// Data quality confidence at source [0.0, 1.0] — the relay fidelity seed
    pub source_confidence: f64,
}

impl Provenance {
    /// Create a new provenance record.
    #[must_use]
    pub fn new(source: DataSource, query: QueryParams, loaded_at: i64, confidence: f64) -> Self {
        Self {
            source,
            query,
            loaded_at,
            source_confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create provenance for internally-generated cargo (e.g., signal pipeline output).
    #[must_use]
    pub fn internal(component: impl Into<String>, loaded_at: i64) -> Self {
        Self {
            source: DataSource::Internal(component.into()),
            query: QueryParams::empty(),
            loaded_at,
            source_confidence: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_creation() {
        let mut query = QueryParams::empty();
        query.insert("drug", "metformin");
        query.insert("event", "lactic acidosis");

        let prov = Provenance::new(DataSource::Faers, query, 1709856000, 0.98);

        assert_eq!(prov.source, DataSource::Faers);
        assert_eq!(prov.query.params.len(), 2);
        assert!((prov.source_confidence - 0.98).abs() < f64::EPSILON);
    }

    #[test]
    fn test_confidence_clamping() {
        let prov = Provenance::new(DataSource::Faers, QueryParams::empty(), 0, 1.5);
        assert!((prov.source_confidence - 1.0).abs() < f64::EPSILON);

        let prov = Provenance::new(DataSource::Faers, QueryParams::empty(), 0, -0.3);
        assert!(prov.source_confidence.abs() < f64::EPSILON);
    }

    #[test]
    fn test_internal_provenance() {
        let prov = Provenance::internal("signal-pipeline", 1709856000);
        assert!(matches!(prov.source, DataSource::Internal(_)));
        assert!((prov.source_confidence - 1.0).abs() < f64::EPSILON);
    }
}
