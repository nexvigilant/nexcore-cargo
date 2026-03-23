//! # Station Stamp — Chain of Custody
//!
//! Every station that handles cargo stamps it: who handled it, what operation
//! was performed, when, and at what fidelity. The custody chain is append-only —
//! stamps accumulate as cargo moves through the system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A custody stamp applied when cargo passes through a station.
///
/// Analogous to a customs stamp in international freight: records who
/// handled the cargo, what was done, and the quality of handling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StationStamp {
    /// Station identifier (e.g., "nexvigilant-station::openfda", "signal-pipeline::detect")
    pub station_id: String,
    /// Operation performed at this station (e.g., "search_adverse_events", "prr_compute")
    pub operation: String,
    /// Unix timestamp when the stamp was applied
    pub stamped_at: i64,
    /// Relay fidelity at this hop [0.0, 1.0]
    pub fidelity: f64,
}

impl StationStamp {
    /// Create a new station stamp.
    #[must_use]
    pub fn new(
        station_id: impl Into<String>,
        operation: impl Into<String>,
        stamped_at: i64,
        fidelity: f64,
    ) -> Self {
        Self {
            station_id: station_id.into(),
            operation: operation.into(),
            stamped_at,
            fidelity: fidelity.clamp(0.0, 1.0),
        }
    }
}

impl fmt::Display for StationStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}::{} (F={:.3})",
            self.station_id, self.operation, self.fidelity
        )
    }
}

/// An ordered chain of station stamps — the cargo's chain of custody.
///
/// Append-only: stamps can be added but not removed or reordered.
/// The chain tracks cumulative fidelity (product of all hop fidelities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustodyChain {
    stamps: Vec<StationStamp>,
}

impl CustodyChain {
    /// Create an empty custody chain.
    #[must_use]
    pub fn new() -> Self {
        Self { stamps: Vec::new() }
    }

    /// Add a stamp to the chain.
    pub fn stamp(&mut self, stamp: StationStamp) {
        self.stamps.push(stamp);
    }

    /// Get all stamps in order.
    #[must_use]
    pub fn stamps(&self) -> &[StationStamp] {
        &self.stamps
    }

    /// Number of stations this cargo has passed through.
    #[must_use]
    pub fn hop_count(&self) -> usize {
        self.stamps.len()
    }

    /// Cumulative fidelity — product of all hop fidelities.
    ///
    /// This is the relay degradation law: F_total = ∏ F_i.
    /// Returns 1.0 for an empty chain (no hops = no loss).
    #[must_use]
    pub fn cumulative_fidelity(&self) -> f64 {
        self.stamps.iter().map(|s| s.fidelity).product()
    }

    /// Whether the cumulative fidelity meets the safety-critical minimum (0.80).
    #[must_use]
    pub fn meets_safety_threshold(&self) -> bool {
        self.cumulative_fidelity() >= 0.80
    }

    /// The most recent station this cargo passed through, if any.
    #[must_use]
    pub fn last_station(&self) -> Option<&StationStamp> {
        self.stamps.last()
    }
}

impl Default for CustodyChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custody_chain_fidelity() {
        let mut chain = CustodyChain::new();
        chain.stamp(StationStamp::new("ingest", "parse", 1000, 0.98));
        chain.stamp(StationStamp::new("detect", "prr", 1001, 0.93));
        chain.stamp(StationStamp::new("threshold", "apply", 1002, 0.97));

        let expected = 0.98 * 0.93 * 0.97;
        assert!((chain.cumulative_fidelity() - expected).abs() < 1e-10);
        assert_eq!(chain.hop_count(), 3);
    }

    #[test]
    fn test_empty_chain_perfect_fidelity() {
        let chain = CustodyChain::new();
        assert!((chain.cumulative_fidelity() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_safety_threshold() {
        let mut safe_chain = CustodyChain::new();
        safe_chain.stamp(StationStamp::new("a", "op", 0, 0.95));
        safe_chain.stamp(StationStamp::new("b", "op", 0, 0.90));
        assert!(safe_chain.meets_safety_threshold()); // 0.855

        let mut unsafe_chain = CustodyChain::new();
        unsafe_chain.stamp(StationStamp::new("a", "op", 0, 0.70));
        unsafe_chain.stamp(StationStamp::new("b", "op", 0, 0.90));
        assert!(!unsafe_chain.meets_safety_threshold()); // 0.63
    }

    #[test]
    fn test_stamp_display() {
        let stamp = StationStamp::new(
            "nexvigilant-station::openfda",
            "search_adverse_events",
            1000,
            0.98,
        );
        assert_eq!(
            stamp.to_string(),
            "nexvigilant-station::openfda::search_adverse_events (F=0.980)"
        );
    }
}
