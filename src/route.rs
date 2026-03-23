//! # Freight Route — The Bill of Lading
//!
//! A freight route is the planned path for cargo through the system.
//! It maps to microgram chains: each waypoint is a processing station
//! that transforms or inspects the cargo.
//!
//! The route is planned at dispatch time but can be modified in transit
//! (e.g., routing cargo to causality assessment when a signal is detected).

use crate::destination::Destination;
use crate::perishability::Perishability;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Freight priority — determines routing order when multiple cargo items
/// compete for processing capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Fatal/life-threatening cases — skip queues, process immediately
    Expedited,
    /// Serious, non-fatal — standard priority
    Standard,
    /// Periodic reporting batches — process when capacity is available
    Bulk,
}

impl Priority {
    /// Numeric rank for sorting (lower = higher priority).
    #[must_use]
    pub fn rank(&self) -> u8 {
        match self {
            Self::Expedited => 0,
            Self::Standard => 1,
            Self::Bulk => 2,
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expedited => write!(f, "EXPEDITED"),
            Self::Standard => write!(f, "Standard"),
            Self::Bulk => write!(f, "Bulk"),
        }
    }
}

/// A waypoint on a freight route — a station the cargo will pass through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waypoint {
    /// Station identifier
    pub station_id: String,
    /// Expected relay fidelity at this hop
    pub expected_fidelity: f64,
    /// Description of what this station does to the cargo
    pub transformation: String,
}

impl Waypoint {
    /// Create a new waypoint.
    #[must_use]
    pub fn new(
        station_id: impl Into<String>,
        expected_fidelity: f64,
        transformation: impl Into<String>,
    ) -> Self {
        Self {
            station_id: station_id.into(),
            expected_fidelity: expected_fidelity.clamp(0.0, 1.0),
            transformation: transformation.into(),
        }
    }
}

/// A planned freight route — the bill of lading for cargo in transit.
///
/// Maps to microgram chains: `workflow-router → prr-signal →
/// signal-to-causality → naranjo-quick → causality-to-action`
/// becomes a `FreightRoute` with 5 waypoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreightRoute {
    /// First station (loading dock)
    pub origin: String,
    /// Final safety decision target
    pub destination: Destination,
    /// Intermediate processing stations
    pub waypoints: Vec<Waypoint>,
    /// Current perishability (may be upgraded during transit)
    pub perishability: Perishability,
    /// Routing priority
    pub priority: Priority,
}

impl FreightRoute {
    /// Create a new freight route.
    #[must_use]
    pub fn new(
        origin: impl Into<String>,
        destination: Destination,
        perishability: Perishability,
    ) -> Self {
        let priority = match perishability {
            Perishability::Expedited { .. } => Priority::Expedited,
            Perishability::Prompt { .. } => Priority::Standard,
            Perishability::Periodic | Perishability::NonPerishable => Priority::Bulk,
        };

        Self {
            origin: origin.into(),
            destination,
            waypoints: Vec::new(),
            perishability,
            priority,
        }
    }

    /// Add a waypoint to the route.
    pub fn add_waypoint(&mut self, waypoint: Waypoint) {
        self.waypoints.push(waypoint);
    }

    /// Expected total fidelity across the planned route.
    ///
    /// Product of all waypoint fidelities — the relay degradation law.
    #[must_use]
    pub fn expected_fidelity(&self) -> f64 {
        self.waypoints.iter().map(|w| w.expected_fidelity).product()
    }

    /// Number of planned hops.
    #[must_use]
    pub fn hop_count(&self) -> usize {
        self.waypoints.len()
    }

    /// Upgrade perishability and adjust priority accordingly.
    pub fn upgrade_perishability(&mut self, new: Perishability) {
        self.perishability = self.perishability.upgrade(new);
        // Sync priority with perishability
        self.priority = match self.perishability {
            Perishability::Expedited { .. } => Priority::Expedited,
            Perishability::Prompt { .. } => Priority::Standard,
            Perishability::Periodic | Perishability::NonPerishable => Priority::Bulk,
        };
    }
}

impl fmt::Display for FreightRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} → [{}] → {} ({}, F={:.3})",
            self.origin,
            self.waypoints
                .iter()
                .map(|w| w.station_id.as_str())
                .collect::<Vec<_>>()
                .join(" → "),
            self.destination,
            self.perishability,
            self.expected_fidelity()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_creation_and_fidelity() {
        let mut route = FreightRoute::new(
            "openfda",
            Destination::SignalDetection,
            Perishability::Periodic,
        );
        route.add_waypoint(Waypoint::new("ingest", 0.98, "FAERS data parsing"));
        route.add_waypoint(Waypoint::new("detect", 0.93, "PRR/ROR computation"));
        route.add_waypoint(Waypoint::new("threshold", 0.97, "Evans criteria gating"));

        let expected = 0.98 * 0.93 * 0.97;
        assert!((route.expected_fidelity() - expected).abs() < 1e-10);
        assert_eq!(route.hop_count(), 3);
        assert_eq!(route.priority, Priority::Bulk); // Periodic = Bulk
    }

    #[test]
    fn test_perishability_upgrade_syncs_priority() {
        let mut route = FreightRoute::new(
            "openfda",
            Destination::SignalDetection,
            Perishability::Periodic,
        );
        assert_eq!(route.priority, Priority::Bulk);

        route.upgrade_perishability(Perishability::PROMPT_90);
        assert_eq!(route.perishability, Perishability::PROMPT_90);
        assert_eq!(route.priority, Priority::Standard);

        route.upgrade_perishability(Perishability::EXPEDITED_15);
        assert_eq!(route.perishability, Perishability::EXPEDITED_15);
        assert_eq!(route.priority, Priority::Expedited);

        // Cannot downgrade
        route.upgrade_perishability(Perishability::Periodic);
        assert_eq!(route.perishability, Perishability::EXPEDITED_15);
        assert_eq!(route.priority, Priority::Expedited);
    }

    #[test]
    fn test_route_display() {
        let mut route = FreightRoute::new(
            "openfda",
            Destination::SignalDetection,
            Perishability::Periodic,
        );
        route.add_waypoint(Waypoint::new("detect", 0.93, "PRR"));
        route.add_waypoint(Waypoint::new("threshold", 0.97, "Evans"));

        let display = route.to_string();
        assert!(display.contains("openfda"));
        assert!(display.contains("Signal Detection"));
        assert!(display.contains("detect"));
    }
}
